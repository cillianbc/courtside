//! The console session on a made-up match.

#![allow(clippy::expect_used, clippy::unwrap_used)]

use courtside_console::Session;
use courtside_console::snapshot::{Pair, Side};
use vb_grammar::Team;

/// Made-up match: home 1–7 (1 sets), visiting 11–16 and libero 17 (14 sets).
fn file() -> String {
    let head = "[3DATAVOLLEYSCOUT]\r\nFILEFORMAT: 2.0\r\nGENERATOR-PRG: Data Volley\r\n\
        [3MATCH]\r\n04/23/2026;;;;;;;;1252;1;Z;\r\n\
        [3TEAMS]\r\nT1;Harbour City;0;Coach A;;\r\nT2;Northgate;0;Coach B;;\r\n\
        [3SET]\r\nTrue;;;;;;\r\n";
    let mut s = String::from(head);
    s.push_str("[3PLAYERS-H]\r\n");
    for n in 1..=7u8 {
        let zone = if n <= 6 { n.to_string() } else { String::new() };
        s.push_str(&format!(
            "0;{n};{n};{zone};;;;;h{n};Home{n};P;;;2;False;h{n};;\r\n"
        ));
    }
    s.push_str("[3PLAYERS-V]\r\n");
    for n in 11..=17u8 {
        let zone = if n <= 16 {
            (n - 10).to_string()
        } else {
            String::new()
        };
        let libero = if n == 17 { "L" } else { "" };
        s.push_str(&format!(
            "1;{n};{n};{zone};;;;;v{n};Away{n};P;;{libero};2;False;v{n};;\r\n"
        ));
    }
    s.push_str("[3SCOUT]\r\n");
    let codes = [
        "*P01>LUp", "*z1>LUp", "aP14>LUp", "az4>LUp", // lineups
        "*01SM-", "a12RM+", "a14EH+", "a13AH#", "ap00:01", // rally 1
        "az3", "*T", "a12SM=", "*p01:01", // rally 2, after visiting's side-out
        "*z6", "*02SH+", // open rally, after home's side-out
    ];
    for (i, c) in codes.iter().enumerate() {
        s.push_str(&format!("{c};;;;;;;15.30.{i:02};1;;;;;;;;;;;;;;;;;;;\r\n"));
    }
    s
}

fn session_with_match() -> (Session, tempfile::TempDir) {
    let dir = tempfile::tempdir().expect("tempdir");
    let dvw = dir.path().join("demo.dvw");
    std::fs::write(&dvw, file()).expect("write");
    let mut session = Session::new();
    session
        .create_season(&dir.path().join("season.courtside"), "2026/27", 1)
        .expect("season");
    session.import_file(&dvw, 1_000).expect("import");
    (session, dir)
}

#[test]
fn snapshot_of_an_imported_match() {
    let (mut session, _dir) = session_with_match();
    let matches = session.matches().expect("matches");
    assert_eq!(matches.len(), 1);
    assert_eq!(matches[0].home, "Harbour City");
    let id = matches[0].id.parse().expect("id");
    let snap = session.open_match(id, 2_000).expect("open");

    assert_eq!(snap.teams.home.name, "Harbour City");
    assert_eq!(snap.teams.visiting.players.len(), 7);
    assert!(
        snap.teams
            .visiting
            .players
            .iter()
            .any(|p| p.number == 17 && p.libero)
    );
    assert_eq!(snap.set, 1);
    assert_eq!(
        snap.score,
        Pair {
            home: 1,
            visiting: 1
        }
    );
    assert_eq!(snap.serving, Some(Side::Home));
    assert_eq!(
        snap.timeouts,
        Pair {
            home: 1,
            visiting: 0
        }
    );

    // Each team sided out once: home's 2 now serves from zone 1.
    assert_eq!(snap.court.home[0], Some(2));
    assert_eq!(snap.court.visiting[0], Some(12));

    let shape: Vec<(Option<u16>, bool, usize)> = snap
        .groups
        .iter()
        .map(|g| (g.rally, g.closed, g.rows.len()))
        .collect();
    assert_eq!(
        shape,
        [(Some(1), true, 9), (Some(2), true, 4), (Some(3), false, 2)]
    );
    assert_eq!(snap.groups[0].won_by, Some(Side::Visiting));
    assert_eq!(snap.groups[2].rows[1].text, "*02SH+");
    assert_eq!(snap.sets.len(), 1);
    assert_eq!(snap.events, 15);

    let json = serde_json::to_value(&snap).expect("json");
    assert_eq!(json["groups"][0]["rows"][0]["kind"], "lineup");
    assert_eq!(json["serving"], "home");
    assert!(json["setterZone"]["home"].is_number());
}

#[test]
fn needs_a_season_and_a_match() {
    let session = Session::new();
    assert!(session.matches().is_err());
    assert!(session.snapshot().is_err());
}

fn opened() -> (Session, tempfile::TempDir) {
    let (mut session, dir) = session_with_match();
    let id = session.matches().expect("matches")[0]
        .id
        .parse()
        .expect("id");
    session.open_match(id, 2_000).expect("open");
    (session, dir)
}

fn texts(snap: &courtside_console::Snapshot) -> Vec<String> {
    snap.groups
        .iter()
        .flat_map(|g| g.rows.iter().map(|r| r.text.clone()))
        .collect()
}

#[test]
fn a_deciding_touch_writes_the_point() {
    let (mut session, _dir) = opened();
    // Home served (*02SH+); a reception error gives home the point.
    let snap = session.commit("a16RM=", None).expect("commit");
    assert_eq!(
        snap.score,
        Pair {
            home: 2,
            visiting: 1
        }
    );
    assert_eq!(texts(&snap)[15..], ["a16RM=", "*p02:01"]);
    assert_eq!(snap.serving, Some(Side::Home));
    assert!(snap.groups[2].closed);
}

#[test]
fn a_side_out_rotates_the_winner() {
    let (mut session, _dir) = opened();
    session.commit("a16RM+", None).expect("reception");
    // Two codes on one line, as a pair.
    let snap = session.commit("a14EH+ a13AH#", None).expect("attack");
    assert_eq!(
        snap.score,
        Pair {
            home: 1,
            visiting: 2
        }
    );
    let t = texts(&snap);
    assert_eq!(t[t.len() - 4..], ["a14EH+", "a13AH#", "ap01:02", "az2"]);
    // 13 rotates into zone 1 and serves next; setter 14 in zone 2.
    assert_eq!(snap.court.visiting[0], Some(13));
    assert_eq!(snap.setter_zone.visiting, Some(2));
    assert_eq!(snap.serving, Some(Side::Visiting));
}

#[test]
fn a_typed_point_is_kept_and_still_rotates() {
    let (mut session, _dir) = opened();
    let snap = session.commit("ap01:02", None).expect("point");
    let t = texts(&snap);
    assert_eq!(t[t.len() - 2..], ["ap01:02", "az2"]);
}

#[test]
fn unreadable_codes_change_nothing() {
    let (mut session, _dir) = opened();
    let before = session.snapshot().expect("snapshot");
    let err = session.commit("a16RM= x16", None).expect_err("bad code");
    assert!(
        err.to_string()
            .starts_with("x16: Start with the shirt number"),
        "{err}"
    );
    assert!(session.commit("a08K", None).is_err(), "unknown skill");
    assert!(session.commit("5S.a12A", None).is_err(), "no such pair");
    assert_eq!(session.snapshot().expect("snapshot"), before);
}

#[test]
fn commits_are_saved_at_once() {
    let (mut session, dir) = opened();
    let snap = session.commit("a16RM=", None).expect("commit");
    // A new session on the same file, as after a crash.
    let mut reopened = Session::new();
    reopened
        .open_season(&dir.path().join("season.courtside"))
        .expect("open season");
    let id = reopened.matches().expect("matches")[0]
        .id
        .parse()
        .expect("id");
    let again = reopened.open_match(id, 3_000).expect("open");
    assert_eq!(texts(&again), texts(&snap));
    assert_eq!(again.score, snap.score);
}

#[test]
fn preview_reads_codes_back() {
    let (session, _dir) = opened();
    let p = session.preview("a13AH# *T").expect("preview");
    assert!(p.ok);
    assert_eq!(
        p.codes[0].meaning,
        "Northgate #13 attack, high, perfect, wins the point"
    );
    assert_eq!(p.codes[1].meaning, "Timeout Harbour City");
    let p = session.preview("a13K").expect("preview");
    assert!(!p.ok);
    assert_eq!(p.codes[0].error.as_deref(), Some("Unknown skill letter K."));
}

/// The open rally: home served (`*02SH+`). Shorthand is filled in from it.
#[test]
fn shorthand_is_expanded_with_the_rally_as_context() {
    let (mut session, _dir) = opened();
    let p = session.preview("a16 a14 a13#").expect("preview");
    assert!(p.ok);
    let codes: Vec<&str> = p.codes.iter().map(|r| r.text.as_str()).collect();
    assert_eq!(codes, ["a16RH+", "a14EH+", "a13AH#"]);
    assert_eq!(p.codes[0].typed, "a16");
    assert_eq!(
        p.codes[0].filled,
        [
            "skill predicted",
            "type from the serve",
            "evaluation default"
        ]
    );
    assert_eq!(
        p.codes[0].meaning,
        "Northgate #16 reception, high, positive"
    );
    // Enter writes the full codes, then the point and rotation as before.
    let snap = session.commit("a16 a14 a13#", None).expect("commit");
    let t = texts(&snap);
    assert_eq!(
        t[t.len() - 5..],
        ["a16RH+", "a14EH+", "a13AH#", "ap01:02", "az2"]
    );
}

#[test]
fn dot_codes_write_both_halves() {
    let (mut session, _dir) = opened();
    session.commit("a16RH+ a14 a13", None).expect("rally");
    // Home digs it; then home attacks and Northgate's 15 blocks it.
    let snap = session.commit("2 4 3A.a15#", None).expect("block");
    let t = texts(&snap);
    assert_eq!(
        t[t.len() - 6..],
        ["*02DH+", "*04EH+", "*03AH/", "a15BH#", "ap01:02", "az2"]
    );
    // A correction takes shorthand too, with the touch before as context.
    let snap = session
        .correct(
            &snap.groups[2]
                .rows
                .iter()
                .find(|r| r.text == "*02DH+")
                .expect("dig")
                .id,
            "2#",
            None,
        )
        .expect("correct");
    assert!(texts(&snap).contains(&"*02DH#".to_string()));
    assert!(
        session
            .correct(&snap.groups[2].rows[1].id, "5S.a12", None)
            .is_err(),
        "one code at a time"
    );
}

#[test]
fn a_won_set_ends_and_the_next_needs_lineups() {
    let (mut session, dir) = opened();
    // Home serves and wins every rally from 1:1 to 25:1.
    for n in 2..=25 {
        session
            .commit(&format!("*p{n:02}:01"), None)
            .expect("point");
    }
    let snap = session.snapshot().expect("snapshot");
    let t = texts(&snap);
    assert_eq!(t[t.len() - 2..], ["*p25:01", "**1set"]);
    assert_eq!(snap.set, 2);
    assert_eq!(
        snap.score,
        Pair {
            home: 0,
            visiting: 0
        }
    );
    assert_eq!(
        snap.needs_lineup,
        Pair {
            home: true,
            visiting: true
        }
    );
    assert!(snap.sets[0].finished);

    let err = session
        .set_lineup(Team::Home, [1, 2, 3, 4, 5, 5], 1, None)
        .expect_err("duplicate");
    assert_eq!(err.to_string(), "lineup: #5 is in two zones");
    assert!(
        session
            .set_lineup(Team::Visiting, [11, 12, 13, 14, 15, 17], 14, None)
            .is_err(),
        "libero"
    );

    session
        .set_lineup(Team::Home, [2, 3, 4, 5, 6, 1], 1, None)
        .expect("home six");
    let snap = session
        .set_lineup(Team::Visiting, [11, 12, 13, 14, 15, 16], 14, None)
        .expect("visiting six");
    assert_eq!(
        snap.needs_lineup,
        Pair {
            home: false,
            visiting: false
        }
    );
    assert_eq!(snap.court.home, [2, 3, 4, 5, 6, 1].map(Some));
    assert_eq!(
        snap.setter_zone,
        Pair {
            home: Some(6),
            visiting: Some(4)
        }
    );
    let t = texts(&snap);
    assert_eq!(t[t.len() - 3..], ["six", "aP14>LUp", "az4>LUp"]);

    // Play on: no rotation issues in set 2.
    let snap = session.commit("a11SM- *2RM+", None).expect("rally");
    let rotation: Vec<_> = snap
        .issues
        .iter()
        .filter(|i| i.rule == "rotation" && i.severity != "info")
        .collect();
    assert!(rotation.is_empty(), "{rotation:?}");

    // The export has set 2's starting zones from the six.
    let out = dir.path().join("out.dvw");
    session.export_dvw(&out).expect("export");
    let text = String::from_utf8_lossy(&std::fs::read(&out).expect("read")).into_owned();
    let row = text
        .lines()
        .find(|l| l.starts_with("0;1;"))
        .expect("home player 1");
    let fields: Vec<&str> = row.split(';').collect();
    assert_eq!((fields[3], fields[4]), ("1", "6"), "{row}");
}

#[test]
fn timeouts_and_substitutions_count_per_set() {
    let (mut session, _dir) = opened();
    session.commit("aT", None).expect("timeout");
    let snap = session.commit("*c03:07", None).expect("sub");
    assert_eq!(
        snap.timeouts,
        Pair {
            home: 1,
            visiting: 1
        }
    );
    assert_eq!(
        snap.substitutions,
        Pair {
            home: 1,
            visiting: 0
        }
    );
    assert!(snap.court.home.contains(&Some(7)));
}

#[test]
fn undo_takes_back_a_code_with_what_the_console_wrote() {
    let (mut session, _dir) = opened();
    let start = texts(&session.snapshot().expect("snapshot"));
    session.commit("a16RM+", None).expect("reception");
    session.commit("a14EH+ a13AH#", None).expect("attack");
    // a13AH# with its point and rotation, then a14EH+, then a16RM+.
    let snap = session.undo().expect("undo");
    assert_eq!(texts(&snap).last().map(String::as_str), Some("a14EH+"));
    assert_eq!(
        snap.score,
        Pair {
            home: 1,
            visiting: 1
        }
    );
    session.undo().expect("undo");
    let snap = session.undo().expect("undo");
    assert_eq!(texts(&snap), start);

    let snap = session.redo().expect("redo");
    assert_eq!(texts(&snap).last().map(String::as_str), Some("a16RM+"));
    session.redo().expect("redo");
    let snap = session.redo().expect("redo");
    assert_eq!(
        snap.score,
        Pair {
            home: 1,
            visiting: 2
        }
    );
    assert_eq!(snap.court.visiting[0], Some(13));
    // Nothing left to redo; a new code clears redo.
    session.redo().expect("nothing to redo");
    session.undo().expect("undo");
    session.commit("a13AH=", None).expect("new code");
    let snap = session.redo().expect("redo is empty");
    assert_eq!(texts(&snap).last().map(String::as_str), Some("*p02:01"));
}

#[test]
fn corrections_insertions_and_removals() {
    let (mut session, dir) = opened();
    let snap = session.snapshot().expect("snapshot");
    let find = |snap: &courtside_console::Snapshot, text: &str| {
        snap.groups
            .iter()
            .flat_map(|g| &g.rows)
            .find(|r| r.text == text)
            .map(|r| r.id.clone())
            .expect(text)
    };
    // Rally 1: the kill was by 12, not 13.
    let snap = session
        .correct(&find(&snap, "a13AH#"), "a12AH#", None)
        .expect("correct");
    assert!(texts(&snap).contains(&"a12AH#".to_string()));
    assert!(!texts(&snap).contains(&"a13AH#".to_string()));
    let row = snap.groups[0]
        .rows
        .iter()
        .find(|r| r.text == "a12AH#")
        .expect("row");
    assert!(row.edited);
    // A dig was missed before the set.
    let snap = session
        .insert_before(&find(&snap, "a14EH+"), "a11DH+", None)
        .expect("insert");
    let t = texts(&snap);
    let at = t.iter().position(|c| c == "a11DH+").expect("inserted");
    assert_eq!(t[at + 1], "a14EH+");
    // The timeout never happened.
    let snap = session.remove(&find(&snap, "*T")).expect("remove");
    assert!(!texts(&snap).contains(&"*T".to_string()));
    assert_eq!(snap.timeouts.home, 0);

    assert!(
        session
            .correct(&find(&snap, "a12AH#"), "a12AH# *T", None)
            .is_err()
    );
    assert!(session.correct(&find(&snap, "a12AH#"), "x", None).is_err());

    // All of it is saved.
    let mut reopened = Session::new();
    reopened
        .open_season(&dir.path().join("season.courtside"))
        .expect("open season");
    let id = reopened.matches().expect("matches")[0]
        .id
        .parse()
        .expect("id");
    assert_eq!(
        texts(&reopened.open_match(id, 3_000).expect("open")),
        texts(&snap)
    );
}

/// A match coded from scratch: no source file, headers from the season's
/// tables. Its export reads back as the same match.
#[test]
fn a_new_match_codes_and_exports() {
    let (mut session, dir) = session_with_match();
    let teams = session.teams().expect("teams");
    let id = |name: &str| {
        teams
            .iter()
            .find(|t| t.name == name)
            .expect(name)
            .id
            .clone()
    };
    let snap = session
        .new_match(
            &id("Northgate"),
            &id("Harbour City"),
            Some("2026-10-15"),
            5_000,
        )
        .expect("new match");
    assert_eq!(snap.teams.home.name, "Northgate");
    assert_eq!(snap.events, 0);
    assert_eq!(
        snap.needs_lineup,
        Pair {
            home: true,
            visiting: true
        }
    );

    session
        .set_lineup(Team::Home, [11, 12, 13, 14, 15, 16], 14, None)
        .expect("home");
    session
        .set_lineup(Team::Visiting, [1, 2, 3, 4, 5, 6], 1, None)
        .expect("visiting");
    session
        .commit("a01SM- *12RM+ *14EH+ *13AH#", None)
        .expect("rally 1");
    let snap = session.commit("*12SM=", None).expect("rally 2");
    assert_eq!(
        snap.score,
        Pair {
            home: 1,
            visiting: 1
        }
    );
    let errors: Vec<_> = snap
        .issues
        .iter()
        .filter(|i| i.severity == "error")
        .collect();
    assert!(errors.is_empty(), "{errors:?}");

    let out = dir.path().join("new.dvw");
    session.export_dvw(&out).expect("export");
    let bytes = std::fs::read(&out).expect("read");
    let text = String::from_utf8_lossy(&bytes);
    assert!(text.contains("GENERATOR-PRG: Courtside\r\n"));
    // Created 5 s after the epoch; DataVolley's stamp style.
    assert!(text.contains("GENERATOR-DAY: 01/01/1970 00.00.05\r\nGENERATOR-IDP: DVW\r\n"));
    assert!(text.contains("LASTCHANGE-PRG: Courtside\r\n"));
    assert!(text.contains("10/15/2026;"), "written date");
    assert!(text.contains(";;46310;"), "date serial");

    // Read back in another season: same codes, same starting six.
    let mut other = Session::new();
    other
        .create_season(&dir.path().join("other.courtside"), "check", 1)
        .expect("season");
    other.import_file(&out, 6_000).expect("import");
    let m = other.matches().expect("matches")[0].id.parse().expect("id");
    let again = other.open_match(m, 7_000).expect("open");
    let codes = |s: &courtside_console::Snapshot| -> Vec<String> {
        texts(s).into_iter().filter(|t| t != "six").collect()
    };
    let snap = session.snapshot().expect("snapshot");
    assert_eq!(codes(&again), codes(&snap));
    assert_eq!(again.score, snap.score);
    assert_eq!(again.court, snap.court);
}

/// After a crash: a new session reopens the season and match from the
/// resume file, with every committed code.
#[test]
fn resumes_where_the_scout_was() {
    let (mut session, dir) = opened();
    session.commit("a16RM=", None).expect("commit");
    let before = session.snapshot().expect("snapshot");
    let file = dir.path().join("config").join("resume.json");
    session.resume_point().save(&file).expect("save resume");
    drop(session);

    let mut again = Session::new();
    let snap = again
        .resume(&courtside_console::Resume::load(&file), 9_000)
        .expect("resume")
        .expect("a match");
    assert_eq!(texts(&snap), texts(&before));
    // The new session can carry on coding with fresh event ids.
    again.commit("*02SM-", None).expect("commit after resume");

    let nothing = courtside_console::Resume::load(&dir.path().join("missing.json"));
    assert_eq!(Session::new().resume(&nothing, 1).expect("resume"), None);
}

/// The path most scouts take: an empty season, two teams entered by hand,
/// a match coded and exported. No .dvw involved.
#[test]
fn from_an_empty_season_to_an_exported_match() {
    use courtside_console::{PlayerDetail, TeamForm};
    let dir = tempfile::tempdir().expect("tempdir");
    let mut session = Session::new();
    session
        .create_season(&dir.path().join("s.courtside"), "2026/27", 1)
        .expect("season");
    let roster = |base: u8| -> Vec<PlayerDetail> {
        (0..7)
            .map(|i| PlayerDetail {
                id: String::new(),
                number: base + i,
                code: String::new(),
                last_name: format!("Player{}", base + i),
                first_name: "Made Up".into(),
                nickname: String::new(),
                role: Some(match i {
                    0 => 5,
                    6 => 1,
                    _ => 2,
                }),
                libero: false,
                captain: i == 1,
                foreign: false,
                birth_date: None,
                height_cm: Some(190),
            })
            .collect()
    };
    let form = |name: &str| TeamForm {
        name: name.into(),
        coach: "Coach Name".into(),
        ..TeamForm::default()
    };
    let home = session
        .save_team(None, &form("Harbour City"), &roster(1), 2)
        .expect("home team");
    let away = session
        .save_team(None, &form("Northgate"), &roster(11), 3)
        .expect("away team");
    assert_eq!(home.code, "HC");
    assert_eq!(home.players[0].code, "PLA-MAD");
    assert!(away.players[6].libero, "role libero makes a libero");

    let snap = session
        .new_match(&home.id, &away.id, Some("2026-10-20"), 4)
        .expect("match");
    assert_eq!(snap.teams.home.players.len(), 7);
    assert_eq!(snap.teams.home.players[0].role, Some(5));
    session
        .set_lineup(Team::Home, [1, 2, 3, 4, 5, 6], 1, None)
        .expect("six");
    session
        .set_lineup(Team::Visiting, [11, 12, 13, 14, 15, 16], 11, None)
        .expect("six");
    session.commit("*01SM# a12RM=", None).expect("ace");
    // A late arrival for this match only.
    let mut late = session.match_roster(false).expect("roster");
    late.push(PlayerDetail {
        number: 20,
        last_name: "Late".into(),
        ..late[0].clone()
    });
    late.last_mut().unwrap().id = String::new();
    late.last_mut().unwrap().code = String::new();
    let snap = session
        .save_match_roster(false, &late, 5)
        .expect("late player");
    assert_eq!(snap.teams.visiting.players.len(), 8);
    assert_eq!(
        snap.score,
        Pair {
            home: 1,
            visiting: 0
        }
    );
    assert_eq!(
        session.teams().expect("teams")[1].players.len(),
        7,
        "team unchanged"
    );

    let out = dir.path().join("m.dvw");
    session.export_dvw(&out).expect("export");
    let text = String::from_utf8_lossy(&std::fs::read(&out).expect("read")).into_owned();
    assert!(text.contains("\r\nHC;Harbour City;"));
    let setter = text
        .lines()
        .find(|l| l.starts_with("0;1;"))
        .expect("home #1");
    let f: Vec<&str> = setter.split(';').collect();
    assert_eq!((f[3], f[8], f[13]), ("1", "PLA-MAD", "5"), "{setter}");
    assert!(
        text.lines().any(|l| l.starts_with("1;20;")),
        "late player exported"
    );
}

/// Correcting one half of a pair corrects the other, and one undo takes
/// back both. Rally 1 of the fixture starts `*01SM- a12RM+`.
#[test]
fn correcting_one_half_of_a_pair_updates_the_other() {
    let (mut session, _dir) = opened();
    let id = |snap: &courtside_console::Snapshot, text: &str| {
        snap.groups
            .iter()
            .flat_map(|g| &g.rows)
            .find(|r| r.text == text)
            .map(|r| r.id.clone())
            .expect(text)
    };
    let snap = session.snapshot().expect("snapshot");
    // The reception was poor: the serve was good.
    let snap = session
        .correct(&id(&snap, "a12RM+"), "a12RM-", None)
        .expect("correct reception");
    let rally: Vec<&str> = snap.groups[0]
        .rows
        .iter()
        .map(|r| r.text.as_str())
        .collect();
    assert_eq!(rally[4..6], ["*01SM+", "a12RM-"]);
    let serve = snap.groups[0]
        .rows
        .iter()
        .find(|r| r.text == "*01SM+")
        .expect("serve");
    assert!(
        serve.auto && serve.edited,
        "written by the console as a correction"
    );

    // A jump serve, not a float: the reception's type follows.
    let snap = session
        .correct(&id(&snap, "*01SM+"), "*01SQ+", None)
        .expect("correct serve");
    assert!(texts(&snap).contains(&"a12RQ-".to_string()));

    // One undo takes back the serve correction and what followed it.
    let snap = session.undo().expect("undo");
    let t = texts(&snap);
    assert!(
        t.contains(&"*01SM+".to_string()) && t.contains(&"a12RM-".to_string()),
        "{t:?}"
    );

    // A code outside a pair leaves its neighbours alone.
    let before = texts(&snap);
    let snap = session
        .correct(&id(&snap, "a14EH+"), "a14EH#", None)
        .expect("correct set");
    let changed: Vec<_> = texts(&snap)
        .into_iter()
        .zip(before)
        .filter(|(a, b)| a != b)
        .collect();
    assert_eq!(changed, [("a14EH#".to_string(), "a14EH+".to_string())]);
}

#[test]
fn the_command_line_types_with_the_season_settings() {
    let (mut session, _dir) = opened();
    // Default season tables know X5: a fast set to zone 4.
    let p = session.preview("a2AX5").expect("preview");
    assert_eq!(p.codes[0].text, "a02AT+X5~4");

    let mut settings = session.settings().expect("settings");
    settings.typing.evaluations[1] = '!'; // reception
    settings.typing.types[1] = 'M';
    session.save_settings(&settings).expect("save");
    // The open rally's serve is *02SH+: the reception's type still comes
    // from the serve, its evaluation from the new default.
    let p = session.preview("a16").expect("preview");
    assert_eq!(p.codes[0].text, "a16RH!");

    settings.typing.evaluations[1] = 'x';
    assert!(
        session.save_settings(&settings).is_err(),
        "not an evaluation"
    );
    settings.typing.evaluations[1] = '+';
    settings
        .tables
        .setter_calls
        .push(courtside_store::SetterCall {
            code: "X5".into(),
            description: String::new(),
        });
    assert!(
        session.save_settings(&settings).is_err(),
        "setter calls start with K"
    );
}
