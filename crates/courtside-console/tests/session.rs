//! The console session on a made-up match.

#![allow(clippy::expect_used, clippy::unwrap_used)]

use courtside_console::Session;
use courtside_console::snapshot::{Pair, Side};

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
