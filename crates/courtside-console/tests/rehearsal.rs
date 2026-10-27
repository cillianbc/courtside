//! The phase 3 exit test, rehearsed headless: a whole five-set match coded
//! through the console as a scout would (lineups, rallies, substitutions,
//! timeouts, corrections, undo), exported, checked and read back. The
//! exports are left in `target/tmp/rehearsal/` for other programs to open
//! (openvolley's `dv_read` via `tools/parity/run.sh rehearsal`).

#![allow(clippy::expect_used, clippy::unwrap_used)]

use courtside_console::snapshot::{Side, Snapshot};
use courtside_console::{ClockTime, PlayerDetail, Session, TeamForm};
use vb_grammar::Team;

/// A small deterministic random source (no dependency needed).
struct Rng(u64);

impl Rng {
    fn below(&mut self, n: u64) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        (self.0 >> 33) % n
    }
}

fn roster(base: u8) -> Vec<PlayerDetail> {
    // 12 players: setter first, libero last.
    (0..12)
        .map(|i| PlayerDetail {
            id: String::new(),
            number: base + i,
            code: String::new(),
            last_name: format!("Player{}", base + i),
            first_name: "Made Up".into(),
            nickname: String::new(),
            role: Some(match i {
                0 | 6 => 5,
                11 => 1,
                3 | 9 => 4,
                4 => 3,
                _ => 2,
            }),
            libero: i == 11,
            captain: i == 1,
            foreign: false,
            birth_date: None,
            height_cm: Some(190),
            weight_kg: None,
            reach_cm: None,
        })
        .collect()
}

fn prefix(side: Side) -> char {
    match side {
        Side::Home => '*',
        Side::Visiting => 'a',
    }
}

fn other(side: Side) -> Side {
    match side {
        Side::Home => Side::Visiting,
        Side::Visiting => Side::Home,
    }
}

fn court(snap: &Snapshot, side: Side) -> [u8; 6] {
    let c = match side {
        Side::Home => snap.court.home,
        Side::Visiting => snap.court.visiting,
    };
    c.map(|n| n.expect("six on court"))
}

fn setter(snap: &Snapshot, side: Side) -> u8 {
    match side {
        Side::Home => snap.setter.home,
        Side::Visiting => snap.setter.visiting,
    }
    .expect("setter known")
}

/// The first player in `zones` (1–6) who isn't the setter.
fn pick(snap: &Snapshot, side: Side, zones: &[usize]) -> u8 {
    let c = court(snap, side);
    let s = setter(snap, side);
    zones
        .iter()
        .map(|z| c[z - 1])
        .find(|&n| n != s)
        .expect("a player who isn't the setter")
}

/// One rally's codes, won by `winner`.
fn rally(snap: &Snapshot, serving: Side, winner: Side, rng: &mut Rng, libero: [u8; 2]) -> String {
    let (s, r) = (serving, other(serving));
    let (ps, pr) = (prefix(s), prefix(r));
    let server = court(snap, s)[0];
    let lib = match r {
        Side::Home => libero[0],
        Side::Visiting => libero[1],
    };
    let receiver = if rng.below(3) == 0 {
        lib
    } else {
        pick(snap, r, &[6, 5, 1])
    };
    let set_r = setter(snap, r);
    let attacker = pick(snap, r, &[4, 2, 3]);
    let roll = rng.below(100);
    if winner == s {
        match roll {
            0..15 => format!("{ps}{server:02}SH#.{pr}{receiver:02}"),
            15..40 => format!(
                "{ps}{server:02}SH+.{pr}{receiver:02} {pr}{set_r:02}EH+ {pr}{attacker:02}AH="
            ),
            40..65 => {
                let blocker = court(snap, s)[2];
                format!(
                    "{ps}{server:02}SH+.{pr}{receiver:02} {pr}{set_r:02}EH+ {pr}{attacker:02}AH/.{ps}{blocker:02}"
                )
            }
            _ => {
                let digger = pick(snap, s, &[6, 5, 1]);
                let set_s = setter(snap, s);
                let hitter = pick(snap, s, &[4, 2, 3]);
                format!(
                    "{ps}{server:02}SH-.{pr}{receiver:02} {pr}{set_r:02}EH+ {pr}{attacker:02}AH+.{ps}{digger:02}D {ps}{set_s:02}EH+ {ps}{hitter:02}AH#"
                )
            }
        }
    } else if roll < 20 {
        format!("{ps}{server:02}SH=")
    } else {
        format!("{ps}{server:02}SH+.{pr}{receiver:02} {pr}{set_r:02}EH# {pr}{attacker:02}AH#")
    }
}

fn texts(snap: &Snapshot) -> Vec<String> {
    snap.groups
        .iter()
        .flat_map(|g| g.rows.iter().map(|r| r.text.clone()))
        .filter(|t| t != "six" && t != "golden")
        .collect()
}

fn set_scores(snap: &Snapshot) -> Vec<(u8, u8)> {
    snap.sets
        .iter()
        .map(|v| (v.score.home, v.score.visiting))
        .collect()
}

fn read_back(path: &std::path::Path, dir: &std::path::Path) -> Snapshot {
    let mut other = Session::new();
    other
        .create_season(
            &dir.join(format!("check-{}.courtside", stem(path))),
            "check",
            1,
        )
        .expect("season");
    let id = other.import_file(path, 2).expect("import");
    other.open_match(id, 3).expect("open")
}

fn stem(path: &std::path::Path) -> String {
    path.file_stem().unwrap().to_string_lossy().into_owned()
}

#[test]
fn a_whole_match_coded_live() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let mut session = Session::new();
    session
        .create_season(&tmp.path().join("season.courtside"), "2026/27", 1)
        .expect("season");
    let form = |name: &str| TeamForm {
        name: name.into(),
        staff: courtside_console::StaffView {
            coach: courtside_console::PersonView {
                last_name: "Coach".into(),
                first_name: "Name".into(),
            },
            ..Default::default()
        },
        ..TeamForm::default()
    };
    let home = session
        .save_team(None, &form("Harbour City"), &roster(1), 2)
        .expect("home");
    let away = session
        .save_team(None, &form("Northgate"), &roster(21), 3)
        .expect("away");
    session
        .new_match(&home.id, &away.id, Some("2026-10-20"), None, 4)
        .expect("match");

    let mut rng = Rng(20_261_020);
    let libero = [12, 32];
    let mut seconds: u32 = 19 * 3600 + 30 * 60;
    let mut clock = || {
        seconds += 25 + rng_seconds(seconds);
        Some(ClockTime {
            hour: u8::try_from(seconds / 3600 % 24).unwrap(),
            minute: u8::try_from(seconds / 60 % 60).unwrap(),
            second: u8::try_from(seconds % 60).unwrap(),
        })
    };
    // Sets 1, 3 and 5 lean home, 2 and 4 away: a 3–2 match.
    let leans = [
        Side::Home,
        Side::Visiting,
        Side::Home,
        Side::Visiting,
        Side::Home,
    ];
    let mut corrections = 0;
    // Time per read-back and per commit (saved, snapshot built), for
    // `cargo test --release ... -- --nocapture`.
    let mut previews = Vec::new();
    let mut commits = Vec::new();
    let mut rallies = 0;
    let mut snap = session.snapshot().expect("snapshot");
    while !snap.match_over {
        assert!(rallies < 500, "the match never ended");
        let set = snap.set;
        if snap.needs_lineup.home || snap.needs_lineup.visiting {
            let turn = usize::from(set - 1);
            let mut h = [1u8, 2, 3, 4, 5, 6];
            let mut v = [21u8, 22, 23, 24, 25, 26];
            h.rotate_left(turn % 6);
            v.rotate_left((turn * 2) % 6);
            session
                .set_lineup(Team::Home, h, 1, clock())
                .expect("home six");
            snap = session
                .set_lineup(Team::Visiting, v, 21, clock())
                .expect("away six");
        }
        let total = snap.score.home + snap.score.visiting;
        let lean = leans[usize::from(set - 1)];
        // First serve of a set: the leaning team's opponents.
        let serving = snap.serving.unwrap_or(other(lean));

        // Timeouts at 8 and 16 points played; a substitution and its return.
        if total == 8 && snap.timeouts.home == 0 {
            snap = session.commit("*T", clock()).expect("timeout");
        }
        if total == 16 && snap.timeouts.visiting == 0 {
            snap = session.commit("aT", clock()).expect("timeout");
        }
        if total == 10 && snap.substitutions.home == 0 {
            let out = pick(&snap, Side::Home, &[4, 3, 2]);
            snap = session
                .commit(&format!("*c{out:02}:08"), clock())
                .expect("sub");
        }
        if total == 30 && snap.substitutions.home == 1 && court(&snap, Side::Home).contains(&8) {
            let back = snap
                .groups
                .iter()
                .flat_map(|g| &g.rows)
                .rev()
                .filter_map(|r| r.text.strip_prefix("*c"))
                .find_map(|t| t.strip_suffix(":08"))
                .map(str::to_owned)
                .expect("sub code");
            snap = session
                .commit(&format!("*c08:{back}"), clock())
                .expect("sub back");
        }

        let winner = if rng.below(100) < 72 {
            lean
        } else {
            other(lean)
        };
        let line = rally(&snap, serving, winner, &mut rng, libero);
        let started = std::time::Instant::now();
        let _ = session.preview(&line).expect("preview");
        previews.push(started.elapsed());
        let started = std::time::Instant::now();
        snap = session
            .commit(&line, clock())
            .unwrap_or_else(|e| panic!("set {set}, {line}: {e}"));
        commits.push(started.elapsed());
        rallies += 1;

        // Now and then the scout fixes a reception, or undoes and redoes.
        if rallies % 37 == 0 && !snap.match_over {
            let target = snap
                .groups
                .iter()
                .flat_map(|g| &g.rows)
                .rev()
                .find(|r| r.kind == "touch" && r.text.get(3..4) == Some("R"))
                .map(|r| (r.id.clone(), r.text.clone()));
            if let Some((id, text)) = target {
                let fixed = format!("{}!", &text[..text.len() - 1]);
                if fixed != text {
                    snap = session
                        .correct(&id, &fixed, None)
                        .expect("modify reception");
                    corrections += 1;
                }
            }
        }
        if rallies % 53 == 0 && !snap.match_over {
            session.undo().expect("undo");
            snap = session.redo().expect("redo");
        }
    }
    assert!(corrections > 0);
    for (what, times) in [("read-back", &mut previews), ("commit", &mut commits)] {
        times.sort();
        let at = |q: usize| times[(times.len() - 1) * q / 100];
        eprintln!(
            "{what}: {} times, median {:?}, 95% {:?}, max {:?}",
            times.len(),
            at(50),
            at(95),
            at(100),
        );
    }
    let json = serde_json::to_string(&snap).expect("json");
    eprintln!(
        "snapshot: {} events, {} rows, {} KB as JSON",
        snap.events,
        snap.groups.iter().map(|g| g.rows.len()).sum::<usize>(),
        json.len() / 1024
    );
    let scores = set_scores(&snap);
    assert_eq!(scores.len(), 5, "{scores:?}");

    let errors: Vec<_> = snap
        .issues
        .iter()
        .filter(|i| i.severity == "error")
        .collect();
    for i in &snap.issues {
        eprintln!("{} {} {}: {}", i.index, i.severity, i.rule, i.message);
    }
    assert!(errors.is_empty(), "{errors:#?}");

    let out = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join("rehearsal");
    std::fs::create_dir_all(&out).expect("out dir");
    let file = out.join("rehearsal-match.dvw");
    session.export_dvw(&file, false).expect("export");
    let back = read_back(&file, tmp.path());
    assert_eq!(texts(&back), texts(&snap), "codes read back");
    assert_eq!(set_scores(&back), scores, "set scores read back");
    assert_eq!(back.starting, snap.starting, "starting sixes read back");
    assert!(back.match_over);

    // A golden set after it.
    snap = session.start_golden_set(clock()).expect("golden set");
    let golden = snap.set;
    session
        .set_lineup(Team::Home, [1, 2, 3, 4, 5, 6], 1, clock())
        .expect("six");
    snap = session
        .set_lineup(Team::Visiting, [21, 22, 23, 24, 25, 26], 21, clock())
        .expect("six");
    while !snap.match_over {
        let serving = snap.serving.unwrap_or(Side::Home);
        let winner = if rng.below(100) < 60 {
            Side::Visiting
        } else {
            Side::Home
        };
        let line = rally(&snap, serving, winner, &mut rng, libero);
        snap = session.commit(&line, clock()).expect("golden rally");
    }
    let (h, v) = set_scores(&snap)[usize::from(golden) - 1];
    assert_eq!(h.max(v), 15.max(h.min(v) + 2), "golden set to 15");
    let file = out.join("rehearsal-golden.dvw");
    session.export_dvw(&file, false).expect("export");
    let text = String::from_utf8_lossy(&std::fs::read(&file).expect("read")).into_owned();
    let teams: Vec<&str> = text
        .lines()
        .skip_while(|l| *l != "[3TEAMS]")
        .skip(1)
        .take(2)
        .collect();
    let won = |line: &str| line.split(';').nth(2).unwrap_or("").to_owned();
    let regular = &scores;
    let home_sets = regular.iter().filter(|(h, v)| h > v).count().to_string();
    assert_eq!(won(teams[0]), home_sets, "the golden set isn't a set won");
}

/// A few seconds of spread between rallies, from the time so far.
fn rng_seconds(seconds: u32) -> u32 {
    seconds.wrapping_mul(2_654_435_761) >> 28
}
