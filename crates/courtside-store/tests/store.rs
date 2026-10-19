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
    assert_eq!(
        season.export_dvw(id, true).expect("export"),
        FILE.as_bytes()
    );
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
    let (home, visiting) = season.match_teams(id).expect("match teams");
    assert_eq!(
        (home.name.as_str(), visiting.name.as_str()),
        ("Harbour City", "Northgate")
    );
    assert_eq!(visiting.players.len(), 2);
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

#[test]
fn starting_sixes_are_stored() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("season.courtside");
    let mut season = Season::create(&path, "2026/27", 1).expect("create");
    let id = season
        .import_dvw("m.dvw", FILE.as_bytes(), 1_000)
        .expect("import");
    let mut open = season.open_match(id).expect("open");
    let six = vb_engine::Body::StartingSix {
        team: vb_grammar::Team::Visiting,
        court: [16, 1, 2, 3, 4, 5],
    };
    open.game.log.append(
        &mut IdGen::new(2_000),
        NewEvent {
            body: six.clone(),
            video_ms: None,
            clock: None,
            producer: Producer::Keyboard,
        },
    );
    season.save(id, &open.game.log).expect("save");
    drop(season);
    let season = Season::open(&path).expect("reopen");
    let log = season.open_match(id).expect("open").game.log;
    assert_eq!(log.all().last().map(|e| &e.body), Some(&six));
}

/// Turns a season file back into the version 2 layout: no settings,
/// teams without codes, players keyed by team and number, no match
/// rosters.
fn downgrade_to_v2(conn: &rusqlite::Connection) {
    conn.execute_batch(
        "ALTER TABLE match DROP COLUMN tables;
         ALTER TABLE match DROP COLUMN video_path;
         DROP TABLE setting;
         DROP INDEX team_code;
         ALTER TABLE team DROP COLUMN code;
         DROP TABLE match_player;
         CREATE TABLE player_v2 (
             team_id TEXT NOT NULL REFERENCES team(id), number INTEGER NOT NULL,
             last_name TEXT NOT NULL, first_name TEXT NOT NULL, libero INTEGER NOT NULL,
             PRIMARY KEY (team_id, number));
         INSERT INTO player_v2 SELECT team_id, number, last_name, first_name, libero FROM player;
         DROP TABLE player;
         ALTER TABLE player_v2 RENAME TO player;
         PRAGMA user_version = 2;",
    )
    .expect("downgrade to v2");
}

/// A version 1 file (before starting sixes and player codes) opens as the
/// current version, with codes made for its players and match rosters
/// filled in.
/// Version 7 adds where each play went; a version 6 file opens with none.
#[test]
fn version_6_files_get_play_positions() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("season.courtside");
    let id = {
        let mut season = Season::create(&path, "2026/27", 1).expect("create");
        season
            .import_dvw("m.dvw", FILE.as_bytes(), 1_000)
            .expect("import")
    };
    {
        let conn = rusqlite::Connection::open(&path).expect("sqlite");
        conn.execute_batch(
            "ALTER TABLE event DROP COLUMN start_xy;
             ALTER TABLE event DROP COLUMN mid_xy;
             ALTER TABLE event DROP COLUMN end_xy;
             PRAGMA user_version = 6;",
        )
        .expect("downgrade to v6");
    }
    let mut season = Season::open(&path).expect("migrated");
    let mut game = season.open_match(id).expect("load").game;
    let first = game.log.all()[0].id;
    assert!(game.log.all()[0].points.is_empty());
    let points = vb_engine::Points {
        start: Some(4868),
        mid: None,
        end: Some(101),
    };
    game.log.set_points(first, points).expect("set");
    season.save_points(id, &game.log, &[first]).expect("save");
    let game = season.open_match(id).expect("reload").game;
    assert_eq!(game.log.all()[0].points, points);
}

#[test]
fn version_1_files_are_migrated() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("season.courtside");
    let (id, coded) = {
        let mut season = Season::create(&path, "2026/27", 1).expect("create");
        let id = season
            .import_dvw("m.dvw", FILE.as_bytes(), 1_000)
            .expect("import");
        let teams = season.teams().expect("teams");
        let coded = season
            .create_match(&teams[0].id, &teams[1].id, None, 1_500)
            .expect("coded here");
        (id, coded)
    };
    {
        let conn = rusqlite::Connection::open(&path).expect("sqlite");
        downgrade_to_v2(&conn);
        // And the version 1 CHECK constraint on events.
        conn.execute_batch(
            "CREATE TABLE event_v1 AS SELECT * FROM event;
             DROP TABLE event;
             CREATE TABLE event (
                 match_id TEXT NOT NULL REFERENCES match(id), seq INTEGER NOT NULL,
                 id TEXT NOT NULL,
                 kind TEXT NOT NULL CHECK (kind IN ('code', 'comment', 'removed')),
                 text TEXT NOT NULL, video_ms INTEGER, clock TEXT, producer TEXT NOT NULL,
                 supersedes TEXT, before TEXT, source_line INTEGER,
                 PRIMARY KEY (match_id, seq));
             INSERT INTO event SELECT match_id, seq, id, kind, text, video_ms, clock, producer,
                 supersedes, before, source_line FROM event_v1;
             DROP TABLE event_v1;
             PRAGMA user_version = 1;",
        )
        .expect("downgrade to v1");
    }
    let mut season = Season::open(&path).expect("migrated");
    assert_eq!(codes(&season, id).len(), 3);
    let teams = season.teams().expect("teams");
    let codes_made: Vec<&str> = teams
        .iter()
        .flat_map(|t| t.players.iter().map(|p| p.code.as_str()))
        .collect();
    assert_eq!(codes_made.len(), 4);
    assert!(codes_made.iter().all(|c| !c.is_empty()));
    // The imported match's roster is the file's (its own player ids).
    let (home, _) = season.match_roster(id).expect("roster");
    assert_eq!(
        home.iter().map(|p| p.code.as_str()).collect::<Vec<_>>(),
        ["h8", "h12"]
    );
    // The match coded here takes the team rosters.
    let (_, visiting) = season.match_roster(coded).expect("roster");
    assert_eq!(visiting.len(), 2);
    assert!(visiting.iter().any(|p| p.libero && p.number == 1));
    assert!(season.export_dvw(coded, false).is_ok());

    let mut open = season.open_match(id).expect("open");
    open.game.log.append(
        &mut IdGen::new(2_000),
        NewEvent {
            body: vb_engine::Body::StartingSix {
                team: vb_grammar::Team::Home,
                court: [8, 12, 1, 2, 3, 4],
            },
            video_ms: None,
            clock: None,
            producer: Producer::Keyboard,
        },
    );
    season
        .save(id, &open.game.log)
        .expect("a six fits the new table");
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
            season.export_dvw(id, true).expect("export") == direct,
            "{}",
            entry.path
        );
        n += 1;
    }
    assert_eq!(season.matches().expect("matches").len(), n);
}

/// An imported match's roster can be edited for that match: the file's
/// player rows follow, untouched rows stay as they were.
#[test]
fn an_imported_match_roster_can_be_edited() {
    use courtside_store::PlayerInput;
    let mut season = Season::in_memory("2026/27", 1).expect("season");
    let id = season
        .import_dvw("m.dvw", FILE.as_bytes(), 1_000)
        .expect("import");
    let (home, _) = season.match_roster(id).expect("roster");
    let mut rows: Vec<PlayerInput> = home
        .iter()
        .map(|p| PlayerInput {
            id: Some(p.id.clone()),
            number: p.number,
            code: p.code.clone(),
            last_name: p.last_name.clone(),
            first_name: p.first_name.clone(),
            role: p.role,
            libero: p.libero,
            ..PlayerInput::default()
        })
        .collect();
    rows[0].last_name = "Renamed".into();
    rows.push(PlayerInput {
        number: 20,
        last_name: "Late".into(),
        first_name: "Arrival".into(),
        ..PlayerInput::default()
    });
    season
        .save_match_roster(id, true, &rows)
        .expect("edit roster");

    let open = season.open_match(id).expect("open");
    assert!(
        open.game.teams.home.player(20).is_some(),
        "the engine sees #20"
    );
    let text = String::from_utf8_lossy(&season.export_dvw(id, false).expect("export")).into_owned();
    let original = |prefix: &str| {
        FILE.split("\r\n")
            .find(|l| l.starts_with(prefix))
            .map(str::to_owned)
    };
    let line = |prefix: &str| {
        text.split("\r\n")
            .find(|l| l.starts_with(prefix))
            .map(str::to_owned)
    };
    assert!(line("0;8;").expect("#8").contains(";Renamed;"));
    assert_eq!(
        line("0;12;"),
        original("0;12;"),
        "an untouched row is kept as it was"
    );
    let late = line("0;20;").expect("#20");
    assert!(late.contains(";Late;Arrival;"), "{late}");
    assert_eq!(line("1;16;"), original("1;16;"));
}

/// A file listing one shirt twice in a team (a slip the validator reports)
/// still imports: the first player keeps the number.
#[test]
fn a_shirt_listed_twice_imports() {
    let twice = FILE.replace(
        "0;12;2;2;;;;;h12;Placeholder;B;;;2;False;h12;;",
        "0;12;2;2;;;;;h12;Placeholder;B;;;2;False;h12;;\r\n0;8;3;;;;;;h8b;Second;E;;;3;False;h8b;;",
    );
    let mut season = Season::in_memory("2026/27", 1).expect("season");
    let id = season
        .import_dvw("twice.dvw", twice.as_bytes(), 1_000)
        .expect("import");
    let (home, _) = season.match_roster(id).expect("roster");
    let eights: Vec<_> = home.iter().filter(|p| p.number == 8).collect();
    assert_eq!(eights.len(), 1);
    assert_eq!(eights[0].last_name, "Placeholder");
    assert_eq!(codes(&season, id).len(), 3);
}
