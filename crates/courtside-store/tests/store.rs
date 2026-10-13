//! Season database: import, edit, save, reopen and export.

#![allow(clippy::expect_used, clippy::unwrap_used)]

use std::path::Path;

use courtside_store::{Error, Season};
use dvw_codec::Dvw;
use serde::Deserialize;
use vb_engine::{IdGen, Match, NewEvent, Producer};
use vb_grammar::Code;

/// Made-up match: two teams, one rally.
const FILE: &str = "[3DATAVOLLEYSCOUT]\r\nFILEFORMAT: 2.0\r\nGENERATOR-PRG: Data Volley\r\n\
    [3MATCH]\r\n04/23/2026;;;;;;;;1252;1;Z;\r\n\
    [3TEAMS]\r\nT1;Harbour City;0;Coach A;;\r\nT2;Northgate;0;Coach B;;\r\n\
    [3SET]\r\nTrue;;;;;;\r\n\
    [3PLAYERS-H]\r\n0;8;1;1;;;;;h8;Placeholder;A;;;5;False;h8;;\r\n0;12;2;2;;;;;h12;Placeholder;B;;;2;False;h12;;\r\n\
    [3PLAYERS-V]\r\n1;16;1;1;;;;;v16;Placeholder;C;;;2;False;v16;;\r\n1;1;2;;;;;;v1;Placeholder;D;;L;2;False;v1;;\r\n\
    [3SCOUT]\r\n*P08>LUp;;;;;;;15.30.00;1;1;;;;;8;12;;;;;16;;;;;;\r\n\
    *z1>LUp;;;;;;;15.30.01;1;1;;;;;8;12;;;;;16;;;;;;\r\n\
    *08SH-;;;;;;;15.30.10;1;1;;;;;8;12;;;;;16;;;;;;\r\n";

fn new(code: &str) -> NewEvent {
    NewEvent::code(Code::parse(code), Producer::Keyboard)
}

fn codes(season: &Season, id: courtside_store::MatchId) -> Vec<String> {
    season
        .open_match(id)
        .expect("open")
        .game
        .log
        .visible()
        .iter()
        .filter_map(|e| e.code().map(ToString::to_string))
        .collect()
}

#[test]
fn import_then_export_gives_the_file_back() {
    let mut season = Season::in_memory("2026/27", 1).expect("season");
    let id = season
        .import_dvw("m.dvw", FILE.as_bytes(), 1_000)
        .expect("import");
    assert_eq!(season.export_dvw(id).expect("export"), FILE.as_bytes());
    let teams = season.teams().expect("teams");
    assert_eq!(
        teams.iter().map(|t| t.name.as_str()).collect::<Vec<_>>(),
        ["Harbour City", "Northgate"]
    );
    assert!(teams[1].players.iter().any(|p| p.number == 1 && p.libero));
    let summary = &season.matches().expect("matches")[0];
    assert_eq!(
        (
            summary.home.as_str(),
            summary.visiting.as_str(),
            summary.events
        ),
        ("Harbour City", "Northgate", 3)
    );
    assert_eq!(summary.date.as_deref(), Some("2026-04-23"));
}

#[test]
fn edits_survive_save_and_reopen() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("season.courtside");
    let id = {
        let mut season = Season::create(&path, "2026/27", 1).expect("create");
        let id = season
            .import_dvw("m.dvw", FILE.as_bytes(), 1_000)
            .expect("import");
        let mut open = season.open_match(id).expect("open");
        let mut ids = IdGen::new(2_000);
        let log = &mut open.game.log;
        log.append(&mut ids, new("a16RH#"));
        let attack = log.append(&mut ids, new("a16AH#"));
        log.append(&mut ids, new("ap00:01"));
        season.save(id, log).expect("save");
        // Correct the attacker, undo a code, then add another, all before
        // the next save: the store must notice the replaced tail.
        log.correct(&mut ids, attack, new("a01AH#"))
            .expect("correct");
        log.undo();
        log.append(&mut ids, new("a16AQ#"));
        season.save(id, log).expect("save");
        id
    };
    let season = Season::open(&path).expect("reopen");
    assert_eq!(season.name().expect("name"), "2026/27");
    assert_eq!(
        codes(&season, id),
        [
            "*P08>LUp", "*z1>LUp", "*08SH-", "a16RH#", "a16AH#", "ap00:01", "a16AQ#"
        ]
    );
}

#[test]
fn teams_are_shared_across_imports() {
    let mut season = Season::in_memory("2026/27", 1).expect("season");
    season
        .import_dvw("a.dvw", FILE.as_bytes(), 1_000)
        .expect("import");
    season
        .import_dvw("b.dvw", FILE.as_bytes(), 1_000)
        .expect("import");
    assert_eq!(season.teams().expect("teams").len(), 2);
    let matches = season.matches().expect("matches");
    assert_eq!(matches.len(), 2);
    assert_ne!(matches[0].id, matches[1].id);
}

#[test]
fn rejects_other_files() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("other.db");
    rusqlite_free_db(&path);
    assert!(matches!(Season::open(&path), Err(Error::Version(0))));
    let season = Season::in_memory("s", 1).expect("season");
    let missing = "01ARZ3NDEKTSV4RRFFQ69G5FAV".parse().expect("id");
    assert!(matches!(season.open_match(missing), Err(Error::NoMatch(_))));
}

/// An empty file: SQLite opens it as a database with schema version 0.
fn rusqlite_free_db(path: &Path) {
    std::fs::write(path, b"").expect("write");
}

#[derive(Deserialize)]
struct Manifest {
    file: Vec<Entry>,
}

#[derive(Deserialize)]
struct Entry {
    path: String,
}

/// Every corpus `.dvw` goes into one season and comes back as the engine
/// would write it directly. Skips without the corpus unless
/// `COURTSIDE_REQUIRE_CORPUS=1`.
#[test]
fn corpus_round_trips_through_the_store() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../testdata");
    let Ok(text) = std::fs::read_to_string(root.join("manifest.toml")) else {
        assert!(
            std::env::var("COURTSIDE_REQUIRE_CORPUS").as_deref() != Ok("1"),
            "corpus required but missing"
        );
        return;
    };
    let manifest: Manifest = toml::from_str(&text).expect("manifest");
    let mut season = Season::in_memory("corpus", 1).expect("season");
    let mut n = 0;
    for entry in manifest.file.iter().filter(|e| e.path.ends_with(".dvw")) {
        let bytes = std::fs::read(root.join(&entry.path)).expect("read");
        let id = season
            .import_dvw(&entry.path, &bytes, 1_000)
            .expect("import");
        let dvw = Dvw::parse(&bytes).expect("parse");
        let direct = Match::from_dvw(&dvw, &mut IdGen::new(0))
            .to_dvw(&dvw)
            .to_bytes()
            .expect("bytes");
        assert!(
            season.export_dvw(id).expect("export") == direct,
            "{}",
            entry.path
        );
        n += 1;
    }
    assert_eq!(season.matches().expect("matches").len(), n);
}
