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
    assert!(err.to_string().starts_with("x16: Start with *"), "{err}");
    assert!(session.commit("a08R", None).is_err(), "no evaluation");
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
    let p = session.preview("a13A").expect("preview");
    assert!(!p.ok);
    assert_eq!(
        p.codes[0].error.as_deref(),
        Some("Add the evaluation (# + ! - / =).")
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
