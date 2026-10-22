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
    // Its own table, unlike the season's default X5 (fast set to zone 4).
    s.push_str("[3ATTACKCOMBINATION]\r\nX5;3;;Q;Made-up quick;;;;C;;\r\n");
    s.push_str("X6;2;;T;Made-up fast;;;;B;;\r\n");
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
    let p = session.preview("a16 a14E a13#").expect("preview");
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
    let snap = session.commit("a16 a14E a13#", None).expect("commit");
    let t = texts(&snap);
    assert_eq!(
        t[t.len() - 5..],
        ["a16RH+", "a14EH+", "a13AH#", "ap01:02", "az2"]
    );
}

/// Northgate's setter is 14 (`aP14`). An attack straight after the
/// team's reception gets the setter's set; a typed set stands instead.
#[test]
fn the_setters_set_is_added_unless_typed() {
    let (mut session, _dir) = opened();
    let p = session.preview("a16# a8#").expect("preview");
    let codes: Vec<&str> = p.codes.iter().map(|r| r.text.as_str()).collect();
    assert_eq!(codes, ["a16RH#", "a14EH+", "a08AH#"]);
    assert_eq!(p.codes[1].typed, "a8#");
    assert_eq!(p.codes[1].filled, ["set by the team's setter"]);
    let snap = session.commit("a16# a8#", None).expect("commit");
    let t = texts(&snap);
    assert_eq!(
        t[t.len() - 5..t.len() - 1],
        ["a16RH#", "a14EH+", "a08AH#", "ap01:02"]
    );

    // Someone else set: the typed set (default evaluation) and no other.
    let p = session.preview("*1RQ- 3e 8Q#").expect("preview");
    let codes: Vec<&str> = p.codes.iter().map(|r| r.text.as_str()).collect();
    assert_eq!(codes, ["a13SQ+", "*01RQ-", "*03EH+", "*08AQ#"]);
    // The set takes the attack's type; the setter's own attack gets none.
    let p = session.preview("*3R- 8Q#").expect("preview");
    let codes: Vec<&str> = p.codes.iter().map(|r| r.text.as_str()).collect();
    assert_eq!(codes, ["a13SH+", "*03RH-", "*01EQ+", "*08AQ#"]);
    let p = session.preview("*3R+ 1#").expect("preview");
    let codes: Vec<&str> = p.codes.iter().map(|r| r.text.as_str()).collect();
    assert_eq!(codes, ["a13SH-", "*03RH+", "*01AH#"], "a setter dump");
}

/// The scout's report (2026-10-18): after the setter goes off, the set
/// added in front of an attack names the player who came on for them.
#[test]
fn a_substitute_setter_sets() {
    let (mut session, _dir) = opened();
    let snap = session.commit("*c01:07", None).expect("substitution");
    assert!(snap.issues.is_empty(), "{:?}", snap.issues);
    let p = session.preview("*3R- 8Q#").expect("preview");
    let codes: Vec<&str> = p.codes.iter().map(|r| r.text.as_str()).collect();
    assert_eq!(codes, ["*03RH-", "*07EQ+", "*08AQ#"]);
}

/// VolleyStation's shorthand (docs/research/VOLLEYSTATION.md §12): a call
/// with no player is the setter's set, the set's target comes from the
/// attack's combination (the made-up table: X5 → C, X6 → B), and the
/// extended part goes after the evaluation without `~`.
#[test]
fn volleystation_shorthand_fills_the_set() {
    let (mut session, _dir) = opened();
    let p = session.preview("a16# a8X5#T2").expect("preview");
    let codes: Vec<&str> = p.codes.iter().map(|r| r.text.as_str()).collect();
    assert_eq!(codes, ["a16RH#", "a14EQ+~~C", "a08AQ#X5~3~~T2"]);
    session.commit("a16# a8X5#T2", None).expect("commit");
    // Northgate serves; home's setter is 1.
    let p = session.preview("*3R+ K7 9X6#").expect("preview");
    let codes: Vec<&str> = p.codes.iter().map(|r| r.text.as_str()).collect();
    assert_eq!(codes, ["a13SH-", "*03RH+", "*01EH+K7B", "*09AT#X6~2"]);
    let p = session.preview("*3R+ 6K1F# 9X6#").expect("preview");
    let codes: Vec<&str> = p.codes.iter().map(|r| r.text.as_str()).collect();
    assert_eq!(codes[2], "*06EH#K1F", "a typed target stays");
}

#[test]
fn dot_codes_write_both_halves() {
    let (mut session, _dir) = opened();
    session.commit("a16RH+ a14E a13", None).expect("rally");
    // Home digs it (a dig is typed: a bare number there is the
    // counter-attack, #24); then home attacks and Northgate's 15 blocks it.
    let snap = session.commit("2D 4E 3A.a15#", None).expect("block");
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
            "2D#",
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
    session.export_dvw(&out, false).expect("export");
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
    session.export_dvw(&out, false).expect("export");
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
    session.export_dvw(&out, false).expect("export");
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

/// The match file with its scout section replaced by `codes`.
fn file_with(codes: &[&str]) -> String {
    let base = file();
    let head = &base[..base.find("[3SCOUT]").expect("scout")];
    let mut s = format!("{head}[3SCOUT]\r\n");
    for c in codes {
        s.push_str(&format!("{c};;;;;;;;1;;;;;;;;;;;;;;;;;;;\r\n"));
    }
    s
}

fn open_file(text: &str) -> (Session, tempfile::TempDir) {
    let dir = tempfile::tempdir().expect("tempdir");
    let dvw = dir.path().join("m.dvw");
    std::fs::write(&dvw, text).expect("write");
    let mut session = Session::new();
    session
        .create_season(&dir.path().join("s.courtside"), "s", 1)
        .expect("season");
    let id = session.import_file(&dvw, 1_000).expect("import");
    session.open_match(id, 2_000).expect("open");
    (session, dir)
}

#[test]
fn a_lineup_is_corrected_in_place() {
    let (mut session, dir) = opened();
    let snap = session.snapshot().expect("snapshot");
    let start = snap.starting[0].home.clone().expect("home six");
    assert_eq!(start.court, [1, 2, 3, 4, 5, 6].map(Some));
    assert_eq!(start.setter, Some(1));

    // 1 and 2 were the other way round; the setter, 1, starts in zone 2.
    let snap = session
        .correct_lineup(Team::Home, 1, [2, 1, 3, 4, 5, 6], 1, None)
        .expect("correct");
    let start = snap.starting[0].home.clone().expect("home six");
    assert_eq!(start.court, [2, 1, 3, 4, 5, 6].map(Some));
    let t = texts(&snap);
    assert!(t.contains(&"*z2>LUp".to_string()), "{t:?}");
    assert!(!t.contains(&"*z1>LUp".to_string()), "{t:?}");
    assert_eq!(
        snap.starting[0].visiting,
        session.snapshot().expect("s").starting[0].visiting
    );

    // The export has the corrected starting zones.
    let out = dir.path().join("lineup.dvw");
    session.export_dvw(&out, false).expect("export");
    let text = String::from_utf8_lossy(&std::fs::read(&out).expect("read")).into_owned();
    assert!(text.contains("0;1;1;2;"), "#1 starts in zone 2");
    assert!(text.contains("0;2;2;1;"), "#2 starts in zone 1");

    assert!(
        session
            .correct_lineup(Team::Home, 1, [2, 1, 3, 4, 5, 7], 9, None)
            .is_err(),
        "setter not in the six"
    );

    // One undo takes it all back.
    let snap = session.undo().expect("undo");
    assert_eq!(
        snap.starting[0].home.as_ref().expect("home six").court,
        [1, 2, 3, 4, 5, 6].map(Some)
    );
    assert!(texts(&snap).contains(&"*z1>LUp".to_string()));
}

#[test]
fn an_earlier_sets_lineup_is_corrected_after_the_match() {
    let three: Vec<String> = (1..=3u8)
        .flat_map(|s| [format!("*p25:{:02}", 10 + s), format!("**{s}set")])
        .collect();
    let (mut session, dir) = open_file(&file_with(
        &three.iter().map(String::as_str).collect::<Vec<_>>(),
    ));
    assert!(session.snapshot().expect("snapshot").match_over);

    let snap = session
        .correct_lineup(Team::Home, 2, [6, 5, 4, 3, 2, 1], 3, None)
        .expect("set 2");
    assert!(snap.match_over);
    let start = |n: usize| snap.starting[n].home.clone().expect("home six");
    assert_eq!(start(1).court, [6, 5, 4, 3, 2, 1].map(Some));
    assert_eq!(start(1).setter, Some(3));
    assert_eq!(
        start(0).court,
        [1, 2, 3, 4, 5, 6].map(Some),
        "set 1 untouched"
    );

    let out = dir.path().join("sets.dvw");
    session.export_dvw(&out, false).expect("export");
    let text = String::from_utf8_lossy(&std::fs::read(&out).expect("read")).into_owned();
    assert!(
        text.contains("0;6;6;6;1;;"),
        "#6: zone 6 in set 1, zone 1 in set 2"
    );
    assert!(
        session
            .correct_lineup(Team::Home, 5, [6, 5, 4, 3, 2, 1], 3, None)
            .is_err(),
        "no set 5"
    );
}

#[test]
fn no_set_follows_a_won_match() {
    let sets = |n: u8| {
        (1..=n)
            .flat_map(|s| [format!("*p25:{:02}", 10 + s), format!("**{s}set")])
            .collect::<Vec<_>>()
    };
    let three = sets(3);
    let (mut session, _dir) = open_file(&file_with(
        &three.iter().map(String::as_str).collect::<Vec<_>>(),
    ));
    assert!(
        session.commit("*T", None).is_err(),
        "nothing after the match"
    );
    let snap = session.snapshot().expect("snapshot");
    assert!(snap.match_over);
    assert_eq!(snap.set, 3);
    assert_eq!(snap.sets.len(), 3, "no set 4 tab");
    assert!(!snap.needs_lineup.home && !snap.needs_lineup.visiting);

    let two = sets(2);
    let (session, _dir) = open_file(&file_with(
        &two.iter().map(String::as_str).collect::<Vec<_>>(),
    ));
    let snap = session.snapshot().expect("snapshot");
    assert!(!snap.match_over);
    assert_eq!((snap.set, snap.sets.len()), (3, 3));
}

/// The scout's set length goes with the set end, is saved, and is what
/// the export writes (#21).
#[test]
fn a_set_end_keeps_the_scouts_minutes() {
    let (mut session, dir) = open_file(&file_with(&[
        "*P01>LUp", "*z1>LUp", "aP14>LUp", "az4>LUp", "*p24:10",
    ]));
    let mut settings = session.settings().expect("settings");
    settings.rules.confirm_set_end = true;
    session.save_settings(&settings).expect("save");
    assert!(
        session
            .commit("p25:10", None)
            .expect("set point")
            .set_end_due
    );
    session.end_set(None, Some(27)).expect("end set");

    // Saved: a fresh session on the same season sees it.
    let id = session.matches().expect("matches")[0]
        .id
        .parse()
        .expect("id");
    let mut again = Session::new();
    again
        .open_season(&dir.path().join("s.courtside"))
        .expect("season");
    again.open_match(id, 3_000).expect("open");
    let out = dir.path().join("out.dvw");
    again.export_dvw(&out, false).expect("export");
    let text = String::from_utf8_lossy(&std::fs::read(&out).expect("read")).into_owned();
    let set1 = text
        .lines()
        .skip_while(|l| !l.starts_with("[3SET]"))
        .nth(1)
        .expect("set 1 row");
    assert_eq!(set1.split(';').nth(5), Some("27"), "{set1}");
}

#[test]
fn set_ends_wait_for_the_scout_when_the_season_asks() {
    let (mut session, _dir) = open_file(&file_with(&[
        "*P01>LUp", "*z1>LUp", "aP14>LUp", "az4>LUp", "*p24:10",
    ]));
    let mut settings = session.settings().expect("settings");
    settings.rules.confirm_set_end = true;
    settings.rules.substitutions = 8;
    session.save_settings(&settings).expect("save");
    assert_eq!(session.snapshot().expect("snapshot").substitution_limit, 8);

    let snap = session.commit("p25:10", None).expect("set point");
    assert!(snap.set_end_due);
    assert!(!texts(&snap).contains(&"**1set".to_string()), "held");
    let err = session.commit("*01SH-", None).expect_err("set not ended");
    assert!(err.to_string().contains("end it first"), "{err}");

    let snap = session.end_set(None, None).expect("end set");
    assert!(!snap.set_end_due);
    assert!(texts(&snap).contains(&"**1set".to_string()));
    assert_eq!(snap.set, 2);
    assert!(session.end_set(None, None).is_err(), "nothing to end");

    // Undo takes the set end back; it is due again.
    assert!(session.undo().expect("undo").set_end_due);

    settings.rules.substitutions = 0;
    assert!(session.save_settings(&settings).is_err());
}

/// Keys remapped to type text (`]` → `#`) are kept with the season.
#[test]
fn keys_belong_to_the_computer_and_are_checked() {
    use courtside_console::Keys;
    use courtside_store::{KeyBinding, KeyText};
    let dir = tempfile::tempdir().expect("tempdir");
    let file = dir.path().join("config").join("keys.json");
    assert_eq!(Keys::load(&file), None, "no file yet");

    // A season saved before keys moved hands its keys over.
    let (mut session, _season) = opened();
    let mut settings = session.settings().expect("settings");
    let kt = |keys: &str, text: &str| KeyText {
        keys: keys.into(),
        text: text.into(),
    };
    settings.key_text = vec![kt("]", "#"), kt("[", "+")];
    session.save_settings(&settings).expect("save");
    let keys = Keys::from_season(&session.settings().expect("settings"));
    assert_eq!(keys.key_text.len(), 2);
    keys.save(&file).expect("save keys");
    assert_eq!(Keys::load(&file), Some(keys.clone()));

    let mut bad = keys.clone();
    bad.key_text.push(kt("]", "="));
    assert!(bad.save(&file).is_err(), "a key twice");
    let mut bad = keys.clone();
    bad.key_text.push(kt("F2", ""));
    assert!(bad.check().is_err(), "no text");
    let mut bad = keys.clone();
    bad.key_text.push(kt("F3", "a b"));
    assert!(bad.check().is_err(), "a space");
    let mut bad = keys.clone();
    bad.keymap.push(KeyBinding {
        action: "undo".into(),
        keys: "]".into(),
    });
    assert!(bad.check().is_err(), "an action's key");
    assert_eq!(
        Keys::load(&file),
        Some(keys),
        "a refused save keeps the file"
    );
}

/// The id of the row reading `text`.
fn row_id(snap: &courtside_console::Snapshot, text: &str) -> String {
    snap.groups
        .iter()
        .flat_map(|g| &g.rows)
        .rev()
        .find(|r| r.text == text)
        .map(|r| r.id.clone())
        .expect(text)
}

#[test]
fn modifying_the_deciding_code_moves_the_point() {
    let (mut session, _dir) = opened();
    // Home serves at 1:1; the visiting kill is a side-out.
    let snap = session.commit("a12RH+ a14EH+ a13AH#", None).expect("rally");
    let t = texts(&snap);
    assert_eq!(t[t.len() - 2..], ["ap01:02", "az2"]);

    // It was an attack error: home wins, no side-out.
    let snap = session
        .correct(&row_id(&snap, "a13AH#"), "a13AH=", None)
        .expect("modify");
    let t = texts(&snap);
    assert_eq!(t[t.len() - 2..], ["a13AH=", "*p02:01"], "{t:?}");
    assert_eq!((snap.score.home, snap.score.visiting), (2, 1));

    // A change that keeps the result leaves the console's codes alone.
    let events = snap.events;
    let snap = session
        .correct(&row_id(&snap, "a14EH+"), "a14EH#", None)
        .expect("modify set");
    assert_eq!(snap.events, events + 1, "only the correction");

    // One undo per edit.
    session.undo().expect("undo");
    let t = texts(&session.undo().expect("undo"));
    assert_eq!(t[t.len() - 2..], ["ap01:02", "az2"]);
}

#[test]
fn a_set_no_longer_won_loses_its_set_end() {
    let (mut session, _dir) = open_file(&file_with(&[
        "*P01>LUp", "*z1>LUp", "aP14>LUp", "az4>LUp", "*p24:20",
    ]));
    let snap = session.commit("*02SH#", None).expect("ace");
    assert!(texts(&snap).contains(&"**1set".to_string()));
    assert_eq!(snap.set, 2);

    // A serve error, not an ace: 24:21, the set goes on.
    let snap = session
        .correct(&row_id(&snap, "*02SH#"), "*02SH=", None)
        .expect("modify");
    let t = texts(&snap);
    assert!(!t.contains(&"**1set".to_string()), "{t:?}");
    assert_eq!(snap.set, 1);
    assert_eq!((snap.score.home, snap.score.visiting), (24, 21));
    assert_eq!(t[t.len() - 2..], ["ap24:21", "az3"], "side-out rotation");
}

/// The four lineup codes: home setter 1 in zone 1, visiting 14 in zone 4.
const LINEUPS: [&str; 4] = ["*P01>LUp", "*z1>LUp", "aP14>LUp", "az4>LUp"];

#[test]
fn review_fixes_on_rally_endings() {
    let (mut session, _dir) = open_file(&file_with(&LINEUPS));
    // Home serves into the net: visiting side-out, its setter 4 → 3.
    let snap = session.commit("*01SH=", None).expect("serve error");
    let t = texts(&snap);
    assert_eq!(t[t.len() - 2..], ["ap00:01", "az3"]);

    // A penalty point to home while visiting serves: home rotates too.
    let snap = session.commit("*p01:01", None).expect("penalty");
    let t = texts(&snap);
    assert_eq!(t[t.len() - 2..], ["*p01:01", "*z6"], "{t:?}");

    // Removing a console-written rotation sticks.
    let snap = session.remove(&row_id(&snap, "*z6")).expect("remove");
    assert!(!texts(&snap).contains(&"*z6".to_string()));

    // Visiting's setter started in zone 2: its later rotation follows.
    let snap = session
        .correct_lineup(Team::Visiting, 1, [11, 14, 12, 13, 15, 16], 14, None)
        .expect("lineup");
    let t = texts(&snap);
    assert!(t.contains(&"az2>LUp".to_string()), "{t:?}");
    assert!(
        t.contains(&"az1".to_string()) && !t.contains(&"az3".to_string()),
        "{t:?}"
    );
}

/// The console's side-out rotations are written again after an edit, from
/// the rally before it to the set's end, and on request from any rally
/// ("Fix rotations from here"); the scout's own stay.
#[test]
fn rotations_follow_the_rally_winners() {
    let (mut session, _dir) = open_file(&file_with(&LINEUPS));
    session.commit("*01SH=", None).expect("serve error");
    session.commit("a12SH=", None).expect("serve error");
    session.commit("*02SH=", None).expect("serve error");
    let snap = session.commit("a13SH=", None).expect("serve error");
    let t = texts(&snap);
    assert_eq!(
        t[4..],
        [
            "*01SH=", "ap00:01", "az3", "a12SH=", "*p01:01", "*z6", "*02SH=", "ap01:02", "az2",
            "a13SH=", "*p02:02", "*z5"
        ]
    );

    // The scout takes out home's first rotation: that sticks.
    let snap = session.remove(&row_id(&snap, "*z6")).expect("remove");
    assert!(!texts(&snap).contains(&"*z6".to_string()));

    // An edit in a later rally writes the rotations after it again, from
    // the court as it now stands: home's setter moves on from zone 1.
    let snap = session
        .correct(&row_id(&snap, "a13SH="), "a13SM=", None)
        .expect("correct");
    let t = texts(&snap);
    assert_eq!(t[t.len() - 3..], ["a13SM=", "*p02:02", "*z6"]);
    // The set's first rotation fault is the one shown; the rest follow it.
    let rotation: Vec<_> = snap
        .issues
        .iter()
        .filter(|i| i.rule == "rotation")
        .collect();
    let faults: Vec<_> = rotation.iter().filter(|i| i.cause_id.is_none()).collect();
    assert_eq!(faults.len(), 1, "{rotation:?}");
    assert!(
        rotation
            .iter()
            .filter(|i| i.cause_id.is_some())
            .all(|i| i.severity == "info"),
        "{rotation:?}"
    );

    // The preview says what the fix will do and changes nothing.
    let unchanged = texts(&snap);
    let preview = session
        .fix_rotations_preview(&row_id(&snap, "a12SH="))
        .expect("preview");
    assert!(preview.codes > 0, "{preview:?}");
    assert!(preview.issues_after < preview.issues_before, "{preview:?}");
    assert!(preview.from_rally.is_some() && preview.from_rally <= preview.to_rally);
    assert_eq!(texts(&session.snapshot().expect("snapshot")), unchanged);

    // Fixing from the second rally puts home's first rotation back and
    // moves the later one on again; one undo takes the fix back alone.
    let snap = session
        .fix_rotations(&row_id(&snap, "a12SH="))
        .expect("fix");
    let t = texts(&snap);
    assert_eq!(t[7..10], ["a12SH=", "*p01:01", "*z6"]);
    assert_eq!(t[t.len() - 3..], ["a13SM=", "*p02:02", "*z5"]);
    assert!(
        snap.issues.iter().all(|i| i.rule != "rotation"),
        "{:?}",
        snap.issues
    );
    let snap = session.undo().expect("undo");
    let t = texts(&snap);
    assert_eq!(t[t.len() - 3..], ["a13SM=", "*p02:02", "*z6"]);
    assert!(!t[..9].contains(&"*z6".to_string()));

    // A rotation the scout typed stands, even where it disagrees.
    let snap = session.commit("*z3", None).expect("typed rotation");
    let at = row_id(&snap, "*z3");
    let snap = session.fix_rotations(&at).expect("fix");
    assert!(texts(&snap).contains(&"*z3".to_string()));
}

#[test]
fn a_confirmed_set_end_follows_a_modified_deciding_code() {
    let mut codes = LINEUPS.to_vec();
    codes.push("*p24:10");
    let (mut session, _dir) = open_file(&file_with(&codes));
    let mut settings = session.settings().expect("settings");
    settings.rules.confirm_set_end = true;
    session.save_settings(&settings).expect("save");
    let snap = session.commit("*01SH#", None).expect("ace");
    assert!(snap.set_end_due);
    let snap = session.end_set(None, None).expect("end set");
    assert_eq!(snap.set, 2);

    let snap = session
        .correct(&row_id(&snap, "*01SH#"), "*01SH=", None)
        .expect("modify");
    assert_eq!(
        (snap.set, snap.score.home, snap.score.visiting),
        (1, 24, 11)
    );
    assert!(!texts(&snap).contains(&"**1set".to_string()));
    // Back to the ace: the set is won again and waits to be confirmed.
    let snap = session
        .correct(&row_id(&snap, "*01SH="), "*01SH#", None)
        .expect("modify back");
    assert!(snap.set_end_due);
}

#[test]
fn an_empty_combination_table_types_with_the_seasons() {
    let text = file_with(&LINEUPS);
    let start = text.find("[3ATTACKCOMBINATION]").expect("table");
    let end = text.find("[3SCOUT]").expect("scout");
    let text = format!("{}{}", &text[..start], &text[end..]);
    let (session, _dir) = open_file(&text);
    let p = session.preview("a2AX5").expect("preview");
    assert_eq!(p.codes[0].text, "a02AT+X5~4", "the season's X5");
}

#[test]
fn a_golden_set_after_the_match() {
    let three: Vec<String> = (1..=3u8)
        .flat_map(|s| [format!("*p25:{:02}", 10 + s), format!("**{s}set")])
        .collect();
    let (mut session, dir) = open_file(&file_with(
        &three.iter().map(String::as_str).collect::<Vec<_>>(),
    ));
    assert!(session.snapshot().expect("snapshot").match_over);

    let snap = session.start_golden_set(None).expect("golden set");
    assert!(!snap.match_over);
    assert_eq!((snap.golden_set, snap.set), (Some(4), 4));
    assert!(snap.needs_lineup.home && snap.needs_lineup.visiting);
    assert!(session.start_golden_set(None).is_err(), "only one");

    session
        .set_lineup(Team::Home, [1, 2, 3, 4, 5, 6], 1, None)
        .expect("home six");
    session
        .set_lineup(Team::Visiting, [11, 12, 13, 14, 15, 16], 14, None)
        .expect("away six");
    // 15 wins a golden set, not 25.
    let snap = session.commit("p14:13", None).expect("14:13");
    assert!(!snap.match_over);
    let snap = session.commit("p15:13", None).expect("15:13");
    assert!(
        texts(&snap).contains(&"**4set".to_string()),
        "set end written"
    );
    assert!(snap.match_over);
    assert!(session.start_golden_set(None).is_err(), "only one");

    // The golden set doesn't count as a set won.
    let out = dir.path().join("golden.dvw");
    session.export_dvw(&out, false).expect("export");
    let text = String::from_utf8_lossy(&std::fs::read(&out).expect("read")).into_owned();
    assert!(text.contains("T1;Harbour City;3;"), "{text}");

    // Undo takes the golden set back like any code.
    for _ in 0..5 {
        session.undo().expect("undo");
    }
    let snap = session.snapshot().expect("snapshot");
    assert!(snap.match_over && snap.golden_set.is_none());
}

#[test]
fn a_new_combination_retargets_the_set_before_it() {
    let (mut session, _dir) = opened();
    let snap = session.commit("a12RH+ a14EH+ a13AX5", None).expect("rally");
    let attack = snap
        .groups
        .iter()
        .flat_map(|g| &g.rows)
        .find(|r| r.text == "a13AQ+X5~3")
        .map(|r| r.id.clone())
        .expect("attack");
    // The set goes to the back attacker, as X6 says.
    let snap = session.correct(&attack, "a13AX6", None).expect("correct");
    let t = texts(&snap);
    assert!(t.contains(&"a14EH+~~B".to_string()), "{t:?}");
    assert!(t.contains(&"a13AT+X6~2".to_string()), "{t:?}");
    // One undo takes back both.
    let t = texts(&session.undo().expect("undo"));
    assert!(t.contains(&"a14EH+".to_string()) && t.contains(&"a13AQ+X5~3".to_string()));
}

#[test]
fn the_command_line_types_with_the_season_settings() {
    let (mut session, _dir) = opened();
    // The imported match types with its own combinations: its X5 is a
    // quick from zone 3, the season's default X5 a fast set to zone 4.
    let p = session.preview("a2AX5").expect("preview");
    assert_eq!(p.codes[0].text, "a02AQ+X5~3");

    let mut settings = session.settings().expect("settings");
    settings.typing.evaluations[1] = '!'; // reception
    settings.typing.types[1] = 'M';
    session.save_settings(&settings).expect("save");
    // The open rally's serve is *02SH+: the reception's type still comes
    // from the serve, its evaluation from the new default.
    let p = session.preview("a16").expect("preview");
    assert_eq!(p.codes[0].text, "a16RH!");
    // The season's combinations reach new matches, not this one.
    let p = session.preview("a2AX5").expect("preview");
    assert_eq!(p.codes[0].text, "a02AQ+X5~3");

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

/// Each row's text and video time.
fn video_times(snap: &courtside_console::Snapshot) -> Vec<(String, Option<u64>)> {
    snap.groups
        .iter()
        .flat_map(|g| g.rows.iter().map(|r| (r.text.clone(), r.video_ms)))
        .collect()
}

fn video_of(snap: &courtside_console::Snapshot, text: &str) -> Option<u64> {
    video_times(snap)
        .into_iter()
        .rev()
        .find(|(t, _)| t == text)
        .and_then(|(_, v)| v)
}

#[test]
fn exact_mode_stamps_each_code_with_its_keystroke() {
    let (mut session, _dir) = opened();
    let snap = session
        .commit_at(
            "a16RM+ a14EH+ a13AH#",
            None,
            &[Some(10_000), Some(11_200), Some(12_050)],
        )
        .expect("commit");
    let v = video_times(&snap);
    assert_eq!(
        v[v.len() - 5..],
        [
            ("a16RM+".into(), Some(10_000)),
            ("a14EH+".into(), Some(11_200)),
            ("a13AH#".into(), Some(12_050)),
            // What the console writes gets the last code's time.
            ("ap01:02".into(), Some(12_050)),
            ("az2".into(), Some(12_050)),
        ]
    );
    // A correction keeps the code's time; an insertion takes the time of
    // the code it goes before.
    let id = |snap: &courtside_console::Snapshot, text: &str| {
        snap.groups
            .iter()
            .flat_map(|g| &g.rows)
            .rev()
            .find(|r| r.text == text)
            .map(|r| r.id.clone())
            .expect("row")
    };
    let snap = session
        .correct(&id(&snap, "a14EH+"), "a14EH#", None)
        .expect("correct");
    assert_eq!(video_of(&snap, "a14EH#"), Some(11_200));
    let snap = session
        .insert_before(&id(&snap, "a13AH#"), "*05BH-", None)
        .expect("insert");
    assert_eq!(video_of(&snap, "*05BH-"), Some(12_050));
}

#[test]
fn predictive_mode_steps_from_the_rallys_first_touch() {
    let (mut session, _dir) = opened();
    let mut settings = session.settings().expect("settings");
    settings.video.predictive = true;
    settings.video.step_ms = 1_500;
    session.save_settings(&settings).expect("save");
    // Close the open rally, then code a new one with video.
    session.commit("a16RM=", None).expect("point");
    session
        .commit_at("*02SM+", None, &[Some(20_000)])
        .expect("serve");
    let snap = session
        .commit_at(
            "a16RM+ a14EH+ a13AH#",
            None,
            &[Some(26_000), Some(26_100), Some(26_200)],
        )
        .expect("rally");
    assert_eq!(video_of(&snap, "*02SM+"), Some(20_000));
    assert_eq!(video_of(&snap, "a16RM+"), Some(21_500));
    assert_eq!(video_of(&snap, "a14EH+"), Some(23_000));
    assert_eq!(video_of(&snap, "a13AH#"), Some(24_500));
}

#[test]
fn a_live_match_syncs_to_its_video_by_the_first_serve() {
    let (mut session, dir) = opened();
    // Coded live: times of day only (15.30.ss, ss = line number).
    let snap = session.sync_video(100_000).expect("sync");
    assert_eq!(video_of(&snap, "*01SM-"), Some(100_000));
    assert_eq!(video_of(&snap, "*02SH+"), Some(110_000));
    assert_eq!(video_of(&snap, "*P01>LUp"), Some(96_000));
    // Again: everything moves with the serve.
    let snap = session.sync_video(50_000).expect("sync again");
    assert_eq!(video_of(&snap, "*01SM-"), Some(50_000));
    assert_eq!(video_of(&snap, "*02SH+"), Some(60_000));
    assert_eq!(video_of(&snap, "*P01>LUp"), Some(46_000));

    // Saved, and written to the .dvw video column in whole seconds.
    let id = snap.match_id.parse().expect("id");
    let snap = session.open_match(id, 3_000).expect("reopen");
    assert_eq!(video_of(&snap, "*02SH+"), Some(60_000));
    let out = dir.path().join("synced.dvw");
    session.export_dvw(&out, false).expect("export");
    let text = String::from_utf8_lossy(&std::fs::read(&out).expect("read")).into_owned();
    let serve = text
        .lines()
        .find(|l| l.starts_with("*01SM-"))
        .expect("serve line");
    assert_eq!(serve.split(';').nth(12), Some("50"));
}

#[test]
fn sync_needs_a_serve() {
    let dir = tempfile::tempdir().expect("tempdir");
    let mut session = Session::new();
    session
        .create_season(&dir.path().join("season.courtside"), "2026/27", 1)
        .expect("season");
    assert!(session.sync_video(1_000).is_err());
}

#[test]
fn the_video_path_is_exported_only_when_asked() {
    let dir = tempfile::tempdir().expect("tempdir");
    let dvw = dir.path().join("demo.dvw");
    let with_video = file().replace(
        "[3SCOUT]\r\n",
        "[3VIDEO]\r\nCamera0=C:\\video\\made-up.mp4\r\n[3SCOUT]\r\n",
    );
    std::fs::write(&dvw, with_video).expect("write");
    let mut session = Session::new();
    session
        .create_season(&dir.path().join("season.courtside"), "2026/27", 1)
        .expect("season");
    let id = session.import_file(&dvw, 1_000).expect("import");
    let snap = session.open_match(id, 2_000).expect("open");
    assert_eq!(snap.video_path.as_deref(), Some("C:\\video\\made-up.mp4"));

    let video_section = |session: &Session, with: bool| {
        let out = dir.path().join("out.dvw");
        session.export_dvw(&out, with).expect("export");
        let text = String::from_utf8_lossy(&std::fs::read(&out).expect("read")).into_owned();
        let at = text.find("[3VIDEO]").expect("section");
        let end = text[at..].find("[3SCOUT]").expect("scout") + at;
        text[at..end].to_owned()
    };
    assert_eq!(video_section(&session, false), "[3VIDEO]\r\n");
    assert_eq!(
        video_section(&session, true),
        "[3VIDEO]\r\nCamera0=C:\\video\\made-up.mp4\r\n"
    );

    // A video opened with the match replaces it, and is kept.
    let snap = session.set_video(Some("/clips/other.mov")).expect("set");
    assert_eq!(snap.video_path.as_deref(), Some("/clips/other.mov"));
    let snap = session.open_match(id, 3_000).expect("reopen");
    assert_eq!(snap.video_path.as_deref(), Some("/clips/other.mov"));
    assert_eq!(
        video_section(&session, true),
        "[3VIDEO]\r\nCamera0=/clips/other.mov\r\n"
    );
}

fn clip_texts(clips: &courtside_console::Clips) -> Vec<&str> {
    clips.clips.iter().map(|c| c.text.as_str()).collect()
}

#[test]
fn clip_playlists_filter_touches_with_video() {
    use courtside_console::ClipFilter;
    use courtside_console::clips::ClipPhase;

    let (mut session, _dir) = opened();
    // Coded live and not synced yet: nothing to play, but the scout is told.
    let all = session.clips(&ClipFilter::default()).expect("clips");
    assert!(all.clips.is_empty());
    assert_eq!(all.without_video, 6);

    // Synced: line i is at 100 s + (i - 4) s.
    session.sync_video(100_000).expect("sync");
    let all = session.clips(&ClipFilter::default()).expect("clips");
    assert_eq!(
        clip_texts(&all),
        ["*01SM-", "a12RM+", "a14EH+", "a13AH#", "a12SM=", "*02SH+"]
    );
    assert_eq!(all.without_video, 0);

    let attack = session
        .clips(&ClipFilter {
            skill: Some('A'),
            ..ClipFilter::default()
        })
        .expect("attacks");
    let a = &attack.clips[0];
    assert_eq!(a.text, "a13AH#");
    assert_eq!(
        (a.video_ms, a.start_ms, a.end_ms),
        (103_000, 100_000, 106_000)
    );
    assert_eq!(a.team, Side::Visiting);
    assert_eq!(
        a.score,
        Pair {
            home: 0,
            visiting: 0
        }
    );

    // Side-out by the visitors: their reception and what follows it.
    let side_out = session
        .clips(&ClipFilter {
            team: Some(Side::Visiting),
            phase: Some(ClipPhase::Reception),
            ..ClipFilter::default()
        })
        .expect("side-out");
    assert_eq!(clip_texts(&side_out), ["a12RM+", "a14EH+", "a13AH#"]);

    let player = session
        .clips(&ClipFilter {
            team: Some(Side::Visiting),
            player: Some(12),
            evaluations: "=".into(),
            ..ClipFilter::default()
        })
        .expect("player");
    assert_eq!(clip_texts(&player), ["a12SM="]);

    // Home's rotation: setter in 1 for the first serve, in 6 after the side-out.
    let rotation = |zone| {
        session
            .clips(&ClipFilter {
                team: Some(Side::Home),
                rotation: Some(zone),
                ..ClipFilter::default()
            })
            .expect("rotation")
    };
    assert_eq!(clip_texts(&rotation(1)), ["*01SM-"]);
    assert_eq!(clip_texts(&rotation(6)), ["*02SH+"]);

    // Pre- and post-roll come from the season settings.
    let mut settings = session.settings().expect("settings");
    settings.video.pre_roll_ms = 1_000;
    settings.video.post_roll_ms = 5_000;
    session.save_settings(&settings).expect("save");
    let attack = session
        .clips(&ClipFilter {
            skill: Some('A'),
            ..ClipFilter::default()
        })
        .expect("attacks");
    assert_eq!(
        (attack.clips[0].start_ms, attack.clips[0].end_ms),
        (102_000, 108_000)
    );
    settings.video.post_roll_ms = 31_000;
    assert!(session.save_settings(&settings).is_err());
}

/// The scout's report (2026-10-19): a rally missed mid-set goes in before
/// the next one; the console writes its point and moves the later scores.
#[test]
fn a_missed_rally_goes_in_mid_set() {
    let (mut session, _dir) = opened();
    let snap = session.snapshot().expect("snapshot");
    let serve = row_id(&snap, "a12SM=");
    let snap = session
        .insert_before(&serve, "a12S# ", None)
        .expect("insert");
    let t = texts(&snap);
    assert_eq!(
        t[9..],
        [
            "az3", "*T", "a12SH#", "ap00:02", "a12SM=", "*p01:02", "*z6", "*02SH+"
        ]
    );
    assert!(
        snap.issues.iter().all(|i| i.severity == "info"),
        "{:?}",
        snap.issues
    );

    // A side-out goes in with the winner's rotation; one undo at a time.
    let serve = row_id(&snap, "a12SM=");
    let snap = session
        .insert_before(&serve, "a12S= ", None)
        .expect("insert");
    let t = texts(&snap);
    assert_eq!(t[13..17], ["a12SH=", "*p01:02", "*z6", "a12SM="]);
    assert_eq!(t[17], "*p02:02");

    // A rally nobody won yet is refused; a typed point ends it and takes
    // the score where it lands.
    let serve = row_id(&snap, "a12SM=");
    assert!(session.insert_before(&serve, "a12S+ 7-", None).is_err());
    let snap = session
        .insert_before(&serve, "a12S+ 7- ap00:00", None)
        .expect("insert");
    let t = texts(&snap);
    // Visiting won it from home's serve: a side-out, so it rotates. The
    // file's own `*z6` after the next point stays (it isn't the console's).
    assert_eq!(t[16..20], ["a12SH+", "*07RH-", "ap01:03", "az2"]);
    assert_eq!(t[20..23], ["a12SM=", "*p02:03", "*z6"]);
}

/// A line of codes goes in inside a rally too, read with the rally there.
#[test]
fn a_line_goes_in_inside_a_rally() {
    let (session, _dir) = opened();
    let snap = session.snapshot().expect("snapshot");
    let attack = row_id(&snap, "a13AH#");
    let p = session
        .preview_before("a14E a10", &attack)
        .expect("preview");
    let codes: Vec<&str> = p.codes.iter().map(|r| r.text.as_str()).collect();
    assert_eq!(codes, ["a14EH+", "a10AH+"]);
}

/// GitHub #12: a rally's first code leaves the serve to the rotation. An
/// evaluation alone is the server's serve; the receiving team's player is
/// the reception of it; anything else is read as typed.
#[test]
fn the_serve_comes_from_the_rotation() {
    let (mut session, _dir) = opened();
    // Rally 3 is home's: #2 is in zone 1 after the side-out.
    let snap = session.snapshot().expect("snapshot");
    let serve = row_id(&snap, "*02SH+");
    let read = |session: &Session, id: &str, text: &str| -> Vec<String> {
        session
            .preview_before(text, id)
            .expect("preview")
            .codes
            .iter()
            .map(|r| r.text.clone())
            .collect()
    };
    assert_eq!(read(&session, &serve, "+"), ["*02SH+"]);
    assert_eq!(read(&session, &serve, "Q#"), ["*02SQ#"]);
    let p = session.preview_before("a7-", &serve).expect("preview");
    let codes: Vec<&str> = p.codes.iter().map(|r| r.text.as_str()).collect();
    assert_eq!(codes, ["*02SH+", "a07RH-"]);
    assert_eq!(p.codes[0].filled[0], "serve by the server in zone 1");
    // The serving team's own player is a serve as typed: the checks judge
    // the server.
    assert_eq!(read(&session, &serve, "7-"), ["*07SH-"]);

    // Live, after a side-out: the winner's new zone-1 player serves.
    let snap = session.commit("a12R# a13#", None).expect("side-out");
    let server = snap.court.visiting[0].expect("zone 1");
    let p = session.preview("7=").expect("preview");
    let codes: Vec<&str> = p.codes.iter().map(|r| r.text.as_str()).collect();
    assert_eq!(codes[0], format!("a{server:02}SH#"));
    assert_eq!(codes[1], "*07RH=");
    // After a point won on serve, the same server again.
    session.commit("+ 7= ", None).expect("ace-ish");
    let p = session.preview("#").expect("preview");
    assert_eq!(p.codes[0].text, format!("a{server:02}SH#"));
}

/// GitHub #11: where a play went on the court. A serve's marks go on the
/// reception too; a click alone starts the play in its start zone; marks
/// are saved, kept by a modify, and exported to columns 4–6.
#[test]
fn plays_are_marked_on_the_court() {
    let (mut session, dir) = opened();
    let snap = session.commit("a12R# a13#", None).expect("rally");
    let serve = row_id(&snap, "*02SH+");
    // From the right of the end line (serve zone 1) to the back left of
    // the other half as its team sees it: zone 1, subzone C.
    let snap = session
        .set_points(&serve, Some((3.0, 0.2)), Some((1.1, 5.9)))
        .expect("mark");
    let row = |snap: &courtside_console::Snapshot, text: &str| {
        snap.groups
            .iter()
            .flat_map(|g| &g.rows)
            .find(|r| r.text == text)
            .cloned()
            .expect(text)
    };
    let s = row(&snap, "*02SH+~~~11C");
    let r = row(&snap, "a12RH#~~~11C");
    assert_eq!(
        (s.start, s.end),
        (r.start, r.end),
        "the reception has the serve's"
    );
    assert_eq!(s.start, Some(vb_engine::court::index(3.0, 0.2)));
    // One change: one undo takes back the marks and both codes' zones.
    let snap = session.undo().expect("undo");
    assert_eq!(row(&snap, "*02SH+").end, None);
    assert_eq!(row(&snap, "a12RH#").end, None);
    let snap = session.redo().expect("redo");
    let serve = row_id(&snap, "*02SH+~~~11C");

    // An attack without a start zone needs a drag; with one, a click, and
    // the mark's zones go over the typed ones.
    let attack = row_id(&snap, "a13AH#");
    assert!(session.set_points(&attack, None, Some((2.0, 5.0))).is_err());
    let snap = session
        .correct(&attack, "a13AH#~~~45", None)
        .expect("zones");
    let attack = row_id(&snap, "a13AH#~~~45");
    let snap = session
        .set_points(&attack, None, Some((1.8, 5.2)))
        .expect("click");
    let a = row(&snap, "a13AH#~~~48A");
    assert_eq!(a.start, Some(vb_engine::court::index(1.0, 3.1)));
    // Clearing puts back what was typed.
    let snap = session.set_points(&a.id, None, None).expect("clear");
    assert_eq!(row(&snap, "a13AH#~~~45").end, None);
    // A drag gives the start zone too; a ball out has no end zone.
    let attack = row_id(&snap, "a13AH#~~~45");
    let snap = session
        .set_points(&attack, Some((3.0, 3.0)), Some((2.0, 6.9)))
        .expect("out");
    let a = row(&snap, "a13AH#~~~2");
    assert!(a.end.is_some());

    // Kept by a modify, saved with the match, written to the .dvw.
    let snap = session.correct(&a.id, "a13AH+~~~2", None).expect("modify");
    assert_eq!(row(&snap, "a13AH+~~~2").end, a.end);
    let id = session.matches().expect("matches")[0]
        .id
        .parse()
        .expect("id");
    let snap = session.open_match(id, 3_000).expect("reopen");
    assert_eq!(row(&snap, "*02SH+~~~11C").end, s.end);
    let out = dir.path().join("marked.dvw");
    session.export_dvw(&out, false).expect("export");
    let text = String::from_utf8_lossy(&std::fs::read(&out).expect("read")).into_owned();
    let line = text
        .lines()
        .find(|l| l.starts_with("*02SH+~~~11C;"))
        .expect("serve line");
    let cols: Vec<&str> = line.split(';').collect();
    let cell = |c: Option<u16>| format!("{:04}", c.expect("cell"));
    assert_eq!(
        cols[4..=6],
        [cell(s.start).as_str(), "-1-1", cell(s.end).as_str()]
    );

    // Clearing the serve clears the reception's too.
    let snap = session.set_points(&serve, None, None).expect("clear");
    assert_eq!(row(&snap, "a12RH#").end, None);
    assert_eq!(row(&snap, "*02SH+").end, None);
}

/// A mark that starts an attack somewhere its combination doesn't is
/// noted: the mark's zone goes in, and one of the two is likely a slip.
#[test]
fn a_mark_against_the_combination_is_noted() {
    let (mut session, _dir) = opened();
    let snap = session.commit("a12R# a13X5#", None).expect("rally");
    let attack = texts(&snap)
        .into_iter()
        .rfind(|t| t.starts_with("a13A"))
        .expect("attack");
    // The match's own table starts X5 in zone 3.
    assert_eq!(attack, "a13AQ#X5~3");
    let id = row_id(&snap, &attack);
    let snap = session
        .set_points(&id, Some((1.0, 3.1)), Some((1.8, 5.2)))
        .expect("mark");
    assert!(
        texts(&snap).contains(&"a13AQ#X5~48A".to_string()),
        "{:?}",
        texts(&snap)
    );
    let note = snap
        .issues
        .iter()
        .find(|i| {
            i.message
                .contains("X5 starts in zone 3, the code in zone 4")
        })
        .expect("note");
    assert_eq!(note.severity, "info");
}

/// A set or block is marked where it was: its zone goes in the end-zone
/// place, as DataVolley writes it.
#[test]
fn a_set_is_marked_where_it_was() {
    let (mut session, _dir) = opened();
    let snap = session.commit("a12R# a14E a13#", None).expect("rally");
    let set = row_id(&snap, "a14EH+");
    let snap = session
        .set_points(&set, None, Some((2.9, 3.2)))
        .expect("click");
    assert!(
        texts(&snap).contains(&"a14EH+~~~~2".to_string()),
        "{:?}",
        texts(&snap)
    );
}
