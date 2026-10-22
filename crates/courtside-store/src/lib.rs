//! The season database (PRD F-01, docs/adr/0001): one SQLite file per
//! season holding teams, players and matches. A match is its event log,
//! one row per [`MatchEvent`], replayed into vb-engine on open; an imported
//! match also keeps its source `.dvw` bytes, which supply the headers on
//! export.
//!
//! The log only grows at the tail or shrinks by undo, so [`Season::save`]
//! writes the difference: rows past the first event that differs are
//! replaced, in one transaction.

mod roster;
mod settings;
mod teams;
mod template;

pub use settings::{
    Combination, KeyBinding, KeyText, Rules, SKILLS, SetterCall, Settings, Tables, Typing, Video,
    WinningRow,
};
pub use teams::{
    Player, PlayerInput, SeasonPlayer, Team, TeamImport, TeamInput, suggest_player_code,
    suggest_team_code,
};

use std::path::Path;

use dvw_codec::Dvw;
use dvw_codec::dvw::VideoEntry;
use rusqlite::{Connection, OptionalExtension, params};
use ulid::Ulid;
use vb_engine::event::{ClockTime, ImportFormat};
use vb_engine::log::{EventLog, LogError};
use vb_engine::{Body, EventId, IdGen, Match, MatchEvent, Points, Producer};
use vb_grammar::Code;
use vb_grammar::Team as Side;

/// Schema version, kept in SQLite's `user_version`.
const VERSION: u32 = 9;

const SCHEMA: &str = "
CREATE TABLE season (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    name TEXT NOT NULL,
    created_ms INTEGER NOT NULL,
    -- Random part of the next team or match id: ids made in the same
    -- millisecond stay distinct without a random number source.
    next_id INTEGER NOT NULL DEFAULT 1
);
CREATE TABLE team (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    code TEXT NOT NULL DEFAULT '',
    coach TEXT NOT NULL,
    assistant TEXT NOT NULL,
    -- The team's colour: a preset's key (apps/desktop teams.ts), '' for the default.
    colour TEXT NOT NULL DEFAULT ''
);
CREATE UNIQUE INDEX team_code ON team(code) WHERE code <> '';
CREATE TABLE match (
    id TEXT PRIMARY KEY,
    home_team_id TEXT NOT NULL REFERENCES team(id),
    visiting_team_id TEXT NOT NULL REFERENCES team(id),
    date TEXT,
    source_name TEXT,
    source_dvw BLOB,
    created_ms INTEGER NOT NULL,
    -- JSON: the season's .dvw tables when a match coded here was created.
    tables TEXT,
    -- The match video's path on this machine (personal data: written
    -- into an export's [3VIDEO] only when the scout asks).
    video_path TEXT
);
CREATE TABLE setting (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);
CREATE TABLE event (
    match_id TEXT NOT NULL REFERENCES match(id),
    seq INTEGER NOT NULL,
    id TEXT NOT NULL,
    -- A starting six's text is the team prefix and six shirt numbers in
    -- zone order, e.g. '* 8 12 13 11 3 14'. 'golden' (empty text): the
    -- next set is a golden set.
    kind TEXT NOT NULL CHECK (kind IN ('code', 'comment', 'removed', 'six', 'golden')),
    text TEXT NOT NULL,
    video_ms INTEGER,
    clock TEXT,
    producer TEXT NOT NULL,
    supersedes TEXT,
    before TEXT,
    source_line INTEGER,
    -- Where the play went on the court: .dvw grid cells (vb_engine::court).
    start_xy INTEGER,
    mid_xy INTEGER,
    end_xy INTEGER,
    -- On a set end: the set's length in minutes as the scout gave it.
    minutes INTEGER,
    PRIMARY KEY (match_id, seq)
);
";

const PLAYER_TABLE_SQL: &str = "
CREATE TABLE player (
    id TEXT PRIMARY KEY,
    team_id TEXT NOT NULL REFERENCES team(id),
    number INTEGER NOT NULL,
    -- Unique in the season; the .dvw player id column.
    code TEXT NOT NULL UNIQUE,
    last_name TEXT NOT NULL,
    first_name TEXT NOT NULL,
    nickname TEXT NOT NULL DEFAULT '',
    -- .dvw role: 1 libero, 2 outside, 3 opposite, 4 middle, 5 setter.
    role INTEGER,
    libero INTEGER NOT NULL,
    captain INTEGER NOT NULL DEFAULT 0,
    foreign_player INTEGER NOT NULL DEFAULT 0,
    birth_date TEXT,
    height_cm INTEGER,
    UNIQUE (team_id, number)
);
";

const MATCH_PLAYER_TABLE_SQL: &str = "
-- Each match's own roster, copied when it is created or imported.
CREATE TABLE match_player (
    match_id TEXT NOT NULL REFERENCES match(id),
    side INTEGER NOT NULL CHECK (side IN (0, 1)),
    number INTEGER NOT NULL,
    player_id TEXT REFERENCES player(id) ON DELETE SET NULL,
    code TEXT NOT NULL,
    last_name TEXT NOT NULL,
    first_name TEXT NOT NULL,
    nickname TEXT NOT NULL,
    role INTEGER,
    libero INTEGER NOT NULL,
    captain INTEGER NOT NULL,
    foreign_player INTEGER NOT NULL,
    PRIMARY KEY (match_id, side, number)
);
";

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Sqlite(#[from] rusqlite::Error),
    #[error(transparent)]
    Dvw(#[from] dvw_codec::Error),
    #[error("the match's event log is damaged: {0}")]
    Log(#[from] LogError),
    #[error("not a Courtside season file (schema version {0}, expected {VERSION})")]
    Version(u32),
    #[error("no match {0} in this season")]
    NoMatch(MatchId),
    #[error("bad stored value: {0}")]
    BadRow(String),
    /// Entered data that can't be saved, in words for the scout.
    #[error("{0}")]
    Invalid(String),
}

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MatchId(pub Ulid);

impl std::fmt::Display for MatchId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl std::str::FromStr for MatchId {
    type Err = Error;
    fn from_str(s: &str) -> Result<Self> {
        Ulid::from_string(s)
            .map(MatchId)
            .map_err(|_| Error::BadRow(format!("match id {s:?}")))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MatchSummary {
    pub id: MatchId,
    pub home: String,
    pub visiting: String,
    /// ISO date, if known.
    pub date: Option<String>,
    /// File name the match was imported from.
    pub source_name: Option<String>,
    pub events: usize,
}

/// A match ready to code: the engine's view and the `.dvw` its headers
/// come from.
#[derive(Debug, Clone)]
pub struct OpenMatch {
    pub id: MatchId,
    pub game: Match,
    /// The `.dvw` its headers come from: the imported file, or a blank one
    /// built from the season's tables for a match coded here.
    pub template: Dvw,
}

pub struct Season {
    conn: Connection,
}

impl Season {
    /// A new season file at `path` (which must not exist yet).
    pub fn create(path: &Path, name: &str, now_ms: u64) -> Result<Self> {
        let conn = Connection::open(path)?;
        Self::init(conn, name, now_ms)
    }

    /// A season that lives only in memory (tests, previews).
    pub fn in_memory(name: &str, now_ms: u64) -> Result<Self> {
        Self::init(Connection::open_in_memory()?, name, now_ms)
    }

    fn init(conn: Connection, name: &str, now_ms: u64) -> Result<Self> {
        let version: u32 = conn.query_row("PRAGMA user_version", [], |r| r.get(0))?;
        if version != 0 {
            return Err(Error::Version(version));
        }
        conn.execute_batch(SCHEMA)?;
        conn.execute_batch(PLAYER_TABLE_SQL)?;
        conn.execute_batch(MATCH_PLAYER_TABLE_SQL)?;
        conn.execute(
            "INSERT INTO season (id, name, created_ms) VALUES (1, ?1, ?2)",
            params![name, ms(now_ms)],
        )?;
        conn.pragma_update(None, "user_version", VERSION)?;
        Self::configure(&conn)?;
        Ok(Season { conn })
    }

    /// An existing season file.
    pub fn open(path: &Path) -> Result<Self> {
        let conn = Connection::open(path)?;
        let version: u32 = conn.query_row("PRAGMA user_version", [], |r| r.get(0))?;
        match version {
            VERSION => {}
            1..=8 => {
                if version < 2 {
                    migrate_1_to_2(&conn)?;
                }
                if version < 3 {
                    migrate_2_to_3(&conn)?;
                }
                if version < 4 {
                    migrate_3_to_4(&conn)?;
                }
                if version < 5 {
                    migrate_4_to_5(&conn)?;
                }
                if version < 6 {
                    migrate_5_to_6(&conn)?;
                }
                if version < 7 {
                    migrate_6_to_7(&conn)?;
                }
                if version < 8 {
                    migrate_7_to_8(&conn)?;
                }
                migrate_8_to_9(&conn)?;
            }
            _ => return Err(Error::Version(version)),
        }
        Self::configure(&conn)?;
        Ok(Season { conn })
    }

    fn configure(conn: &Connection) -> Result<()> {
        // WAL: a crash mid-write leaves the last committed event intact.
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "foreign_keys", true)?;
        conn.pragma_update(None, "synchronous", "FULL")?;
        Ok(())
    }

    pub fn name(&self) -> Result<String> {
        Ok(self
            .conn
            .query_row("SELECT name FROM season", [], |r| r.get(0))?)
    }

    /// Imports a `.dvw` as a match: its teams and players join the season
    /// (a team already in the season, by name, is reused and gains any new
    /// players), every `[3SCOUT]` line becomes an event, and the file is
    /// kept for export.
    pub fn import_dvw(&mut self, file_name: &str, bytes: &[u8], now_ms: u64) -> Result<MatchId> {
        let dvw = Dvw::parse(bytes)?;
        let game = Match::from_dvw(&dvw, &mut IdGen::new(now_ms));
        let tx = self.conn.transaction()?;
        let id = MatchId(next_id(&tx, now_ms)?);
        let mut team_ids = Vec::new();
        let mut rosters = Vec::new();
        for (i, players) in [&dvw.players_home, &dvw.players_visiting]
            .into_iter()
            .enumerate()
        {
            let team = dvw.teams.get(i);
            let name = team
                .map(|t| t.name())
                .filter(|n| !n.trim().is_empty())
                .unwrap_or_else(|| ["Home", "Visiting"][i].to_owned());
            let existing: Option<String> = tx
                .query_row("SELECT id FROM team WHERE name = ?1", [&name], |r| r.get(0))
                .optional()?;
            let team_id = match existing {
                Some(t) => t,
                None => {
                    let t = next_id(&tx, now_ms)?.to_string();
                    tx.execute(
                        "INSERT INTO team (id, name, coach, assistant) VALUES (?1, ?2, ?3, ?4)",
                        params![
                            t,
                            name,
                            team.map(|t| t.coach()).unwrap_or_default(),
                            team.map(|t| t.assistant()).unwrap_or_default()
                        ],
                    )?;
                    t
                }
            };
            let mut rows = Vec::new();
            let mut numbers = std::collections::HashSet::new();
            for p in players {
                let Some(number) = p.number() else { continue };
                // A shirt listed twice (a slip in the file, which the
                // validator reports): the first player keeps it.
                if !numbers.insert(number) {
                    continue;
                }
                let special = p.special_role();
                let row = PlayerInput {
                    id: None,
                    number,
                    code: p.id().trim().to_owned(),
                    last_name: p.last_name(),
                    first_name: p.first_name(),
                    nickname: p.nickname(),
                    role: p.role().map(role_number),
                    libero: special == "L" || p.role() == Some(dvw_codec::dvw::Role::Libero),
                    captain: special == "C",
                    foreign: p.foreign().unwrap_or(false),
                    birth_date: None,
                    height_cm: None,
                };
                let (player_id, code, _) = teams::find_or_add_player(&tx, &team_id, &row, now_ms)?;
                rows.push((player_id, code, row));
            }
            rosters.push(rows);
            team_ids.push(team_id);
        }
        tx.execute(
            "INSERT INTO match (id, home_team_id, visiting_team_id, date, source_name, source_dvw, created_ms, video_path)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                id.to_string(),
                team_ids[0],
                team_ids[1],
                dvw.match_date().map(|(d, _)| d.to_string()),
                file_name,
                bytes,
                ms(now_ms),
                // The file's first camera.
                dvw.video
                    .first()
                    .map(|v| v.path().trim())
                    .filter(|p| !p.is_empty()),
            ],
        )?;
        for (side, rows) in (0u8..).zip(&rosters) {
            for (player_id, code, row) in rows {
                // The file's own id when it has one, as the file says.
                let code = if row.code.is_empty() { code } else { &row.code };
                teams::insert_match_player(&tx, id, side, Some(player_id), row, code)?;
            }
        }
        write_events(&tx, id, game.log.all(), 0)?;
        tx.commit()?;
        Ok(id)
    }

    pub fn matches(&self) -> Result<Vec<MatchSummary>> {
        let mut stmt = self.conn.prepare(
            "SELECT m.id, h.name, v.name, m.date, m.source_name,
                    (SELECT COUNT(*) FROM event e WHERE e.match_id = m.id)
             FROM match m JOIN team h ON h.id = m.home_team_id JOIN team v ON v.id = m.visiting_team_id
             ORDER BY m.date, m.created_ms",
        )?;
        let rows = stmt.query_map([], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get(1)?,
                r.get(2)?,
                r.get(3)?,
                r.get(4)?,
                r.get::<_, i64>(5)?,
            ))
        })?;
        rows.map(|row| {
            let (id, home, visiting, date, source_name, events) = row?;
            Ok(MatchSummary {
                id: id.parse()?,
                home,
                visiting,
                date,
                source_name,
                events: usize::try_from(events).unwrap_or(0),
            })
        })
        .collect()
    }

    /// A match coded in the console, between two of the season's teams.
    /// It has no source file: its `.dvw` headers come from the team tables.
    pub fn create_match(
        &mut self,
        home_team_id: &str,
        visiting_team_id: &str,
        date: Option<&str>,
        now_ms: u64,
    ) -> Result<MatchId> {
        if home_team_id == visiting_team_id {
            return Err(Error::Invalid("a match needs two different teams".into()));
        }
        let tables = serde_json::to_string(&self.settings()?.tables)
            .map_err(|e| Error::Invalid(e.to_string()))?;
        let tx = self.conn.transaction()?;
        let id = MatchId(next_id(&tx, now_ms)?);
        tx.execute(
            "INSERT INTO match (id, home_team_id, visiting_team_id, date, created_ms, tables)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                id.to_string(),
                home_team_id,
                visiting_team_id,
                date,
                ms(now_ms),
                tables
            ],
        )?;
        teams::copy_rosters(&tx, id, home_team_id, visiting_team_id)?;
        tx.commit()?;
        Ok(id)
    }

    /// The `.dvw` a match with no source file starts from.
    fn blank_template(&self, id: MatchId) -> Result<Vec<u8>> {
        let (home, visiting) = self.match_teams(id)?;
        let (date, created): (Option<String>, i64) = self.conn.query_row(
            "SELECT date, created_ms FROM match WHERE id = ?1",
            [id.to_string()],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )?;
        let tables = self.match_tables(id)?;
        let created = u64::try_from(created).unwrap_or(0);
        Ok(template::blank(
            &home,
            &visiting,
            date.as_deref(),
            &tables,
            created,
        ))
    }

    /// Loads a match: the engine's model from its source file, the event
    /// log from the stored rows.
    pub fn open_match(&self, id: MatchId) -> Result<OpenMatch> {
        let source: Option<Option<Vec<u8>>> = self
            .conn
            .query_row(
                "SELECT source_dvw FROM match WHERE id = ?1",
                [id.to_string()],
                |r| r.get(0),
            )
            .optional()?;
        let source = source.ok_or(Error::NoMatch(id))?;
        let imported = source.is_some();
        let bytes = match source {
            Some(bytes) => bytes,
            None => self.blank_template(id)?,
        };
        let mut template = Dvw::parse(&bytes)?;
        if imported {
            // The match's roster as edited in the console.
            let (home, visiting) = self.match_roster(id)?;
            roster::apply(&mut template, &home, &visiting);
        }
        let mut game = Match::from_dvw(&template, &mut IdGen::new(0));
        game.log = EventLog::from_events(read_events(&self.conn, id)?)?;
        Ok(OpenMatch { id, game, template })
    }

    /// Writes the log's changes since the last save, in one transaction.
    pub fn save(&mut self, id: MatchId, log: &EventLog) -> Result<()> {
        let tx = self.conn.transaction()?;
        let stored: Vec<String> = tx
            .prepare("SELECT id FROM event WHERE match_id = ?1 ORDER BY seq")?
            .query_map([id.to_string()], |r| r.get(0))?
            .collect::<rusqlite::Result<_>>()?;
        let events = log.all();
        let same = stored
            .iter()
            .zip(events)
            .take_while(|(s, e)| **s == e.id.to_string())
            .count();
        tx.execute(
            "DELETE FROM event WHERE match_id = ?1 AND seq >= ?2",
            params![id.to_string(), i64::try_from(same).unwrap_or(i64::MAX)],
        )?;
        write_events(&tx, id, &events[same..], same)?;
        tx.commit()?;
        Ok(())
    }

    /// Saves a set end's length in minutes as the log now has it. The
    /// event itself is already saved.
    pub fn save_minutes(
        &mut self,
        id: MatchId,
        event: EventId,
        minutes: Option<u16>,
    ) -> Result<()> {
        self.conn.execute(
            "UPDATE event SET minutes = ?1 WHERE match_id = ?2 AND id = ?3",
            params![minutes, id.to_string(), event.to_string()],
        )?;
        Ok(())
    }

    /// Saves the court positions of the events `ids` as the log now has
    /// them. The events themselves are already saved.
    pub fn save_points(&mut self, id: MatchId, log: &EventLog, ids: &[EventId]) -> Result<()> {
        let tx = self.conn.transaction()?;
        {
            let mut stmt = tx.prepare(
                "UPDATE event SET start_xy = ?1, mid_xy = ?2, end_xy = ?3 WHERE match_id = ?4 AND id = ?5",
            )?;
            for e in log.all().iter().filter(|e| ids.contains(&e.id)) {
                stmt.execute(params![
                    e.points.start,
                    e.points.mid,
                    e.points.end,
                    id.to_string(),
                    e.id.to_string()
                ])?;
            }
        }
        tx.commit()?;
        Ok(())
    }

    /// Saves every event's video time as the log now has it (after a sync
    /// moved them), in one transaction. The events themselves are already
    /// saved.
    pub fn save_video_times(&mut self, id: MatchId, log: &EventLog) -> Result<()> {
        let tx = self.conn.transaction()?;
        {
            let mut stmt =
                tx.prepare("UPDATE event SET video_ms = ?1 WHERE match_id = ?2 AND id = ?3")?;
            for e in log.all() {
                stmt.execute(params![
                    e.video_ms.map(ms),
                    id.to_string(),
                    e.id.to_string()
                ])?;
            }
        }
        tx.commit()?;
        Ok(())
    }

    /// The match video's path, from the imported file's `[3VIDEO]` or the
    /// video last opened with the match.
    pub fn match_video(&self, id: MatchId) -> Result<Option<String>> {
        let path: Option<Option<String>> = self
            .conn
            .query_row(
                "SELECT video_path FROM match WHERE id = ?1",
                [id.to_string()],
                |r| r.get(0),
            )
            .optional()?;
        path.ok_or(Error::NoMatch(id))
    }

    pub fn set_match_video(&mut self, id: MatchId, path: Option<&str>) -> Result<()> {
        let n = self.conn.execute(
            "UPDATE match SET video_path = ?1 WHERE id = ?2",
            params![path, id.to_string()],
        )?;
        if n == 0 {
            return Err(Error::NoMatch(id));
        }
        Ok(())
    }

    /// The match as `.dvw` bytes: headers from its source file, everything
    /// the engine derives regenerated. `[3VIDEO]` holds the match video's
    /// path only `with_video`; otherwise it is empty, also for an imported
    /// file that had one (the path is personal data).
    pub fn export_dvw(&self, id: MatchId, with_video: bool) -> Result<Vec<u8>> {
        let open = self.open_match(id)?;
        let mut template = open.template;
        let path = if with_video {
            self.match_video(id)?
        } else {
            None
        };
        // An imported file's own line stays byte for byte when it is still
        // the match's video.
        let keep = template
            .video
            .first()
            .is_some_and(|v| Some(v.path().trim()) == path.as_deref());
        if !keep {
            template.video = path
                .map(|p| vec![VideoEntry(format!("Camera0={p}"))])
                .unwrap_or_default();
        }
        Ok(open.game.to_dvw(&template).to_bytes()?)
    }
}

/// Version 2 lets event rows hold a starting six. SQLite can't change a
/// CHECK constraint in place, so the table is rebuilt.
fn migrate_1_to_2(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "BEGIN;
         CREATE TABLE event_v2 (
             match_id TEXT NOT NULL REFERENCES match(id),
             seq INTEGER NOT NULL,
             id TEXT NOT NULL,
             kind TEXT NOT NULL CHECK (kind IN ('code', 'comment', 'removed', 'six')),
             text TEXT NOT NULL,
             video_ms INTEGER,
             clock TEXT,
             producer TEXT NOT NULL,
             supersedes TEXT,
             before TEXT,
             source_line INTEGER,
             PRIMARY KEY (match_id, seq)
         );
         INSERT INTO event_v2 SELECT * FROM event;
         DROP TABLE event;
         ALTER TABLE event_v2 RENAME TO event;
         PRAGMA user_version = 2;
         COMMIT;",
    )?;
    Ok(())
}

/// Version 3: players get an id and a season-unique code, plus nickname,
/// role, captain, foreign, birth date and height; teams get a code; each
/// match gets its own roster (from its file, or the team's roster for a
/// match coded here).
fn migrate_2_to_3(conn: &Connection) -> Result<()> {
    conn.execute_batch("BEGIN;")?;
    let result = (|| -> Result<()> {
        conn.execute_batch(&format!(
            "ALTER TABLE team ADD COLUMN code TEXT NOT NULL DEFAULT '';
             CREATE UNIQUE INDEX team_code ON team(code) WHERE code <> '';
             ALTER TABLE player RENAME TO player_v2;
             {PLAYER_TABLE_SQL}
             {MATCH_PLAYER_TABLE_SQL}"
        ))?;
        let old: Vec<(String, u8, String, String, bool)> = conn
            .prepare("SELECT team_id, number, last_name, first_name, libero FROM player_v2")?
            .query_map([], |r| {
                Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?))
            })?
            .collect::<rusqlite::Result<_>>()?;
        let mut taken = std::collections::HashSet::new();
        for (team_id, number, last, first, libero) in old {
            let code = suggest_player_code(&last, &first, number, &taken);
            taken.insert(code.clone());
            let id = next_id(conn, 0)?.to_string();
            conn.execute(
                "INSERT INTO player (id, team_id, number, code, last_name, first_name, role, libero)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![id, team_id, number, code, last, first, libero.then_some(1u8), libero],
            )?;
        }
        conn.execute_batch("DROP TABLE player_v2;")?;
        // Matches coded here take their teams' rosters; imported ones the
        // rosters in their file, linked to season players by number.
        let matches: Vec<(String, String, String, Option<Vec<u8>>)> = conn
            .prepare("SELECT id, home_team_id, visiting_team_id, source_dvw FROM match")?
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)))?
            .collect::<rusqlite::Result<_>>()?;
        for (id, home, visiting, source) in matches {
            let rows: [Vec<PlayerInput>; 2] = match source {
                None => [&home, &visiting].map(|team| {
                    conn.prepare("SELECT number, code, last_name, first_name, libero FROM player WHERE team_id = ?1")
                        .and_then(|mut st| {
                            st.query_map([team], |r| {
                                Ok(PlayerInput {
                                    number: r.get(0)?,
                                    code: r.get(1)?,
                                    last_name: r.get(2)?,
                                    first_name: r.get(3)?,
                                    libero: r.get(4)?,
                                    role: r.get::<_, bool>(4)?.then_some(1),
                                    ..PlayerInput::default()
                                })
                            })?
                            .collect()
                        })
                        .unwrap_or_default()
                }),
                Some(bytes) => {
                    let dvw = Dvw::parse(&bytes)?;
                    [&dvw.players_home, &dvw.players_visiting].map(|ps| {
                        ps.iter()
                            .filter_map(|p| {
                                Some(PlayerInput {
                                    number: p.number()?,
                                    code: p.id().trim().to_owned(),
                                    last_name: p.last_name(),
                                    first_name: p.first_name(),
                                    nickname: p.nickname(),
                                    role: p.role().map(role_number),
                                    libero: p.special_role() == "L"
                                        || p.role() == Some(dvw_codec::dvw::Role::Libero),
                                    captain: p.special_role() == "C",
                                    foreign: p.foreign().unwrap_or(false),
                                    ..PlayerInput::default()
                                })
                            })
                            .collect()
                    })
                }
            };
            for (side, (team, rows)) in (0u8..).zip([&home, &visiting].into_iter().zip(rows)) {
                for r in rows {
                    let (player_id, code): (Option<String>, String) = conn
                        .query_row(
                            "SELECT id, code FROM player WHERE team_id = ?1 AND number = ?2",
                            params![team, r.number],
                            |row| Ok((Some(row.get(0)?), row.get(1)?)),
                        )
                        .optional()?
                        .unwrap_or((None, String::new()));
                    let code = if r.code.is_empty() {
                        code
                    } else {
                        r.code.clone()
                    };
                    conn.execute(
                        "INSERT OR IGNORE INTO match_player (match_id, side, number, player_id, code,
                             last_name, first_name, nickname, role, libero, captain, foreign_player)
                         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
                        params![
                            id, side, r.number, player_id, code, r.last_name, r.first_name,
                            r.nickname, r.role, r.libero, r.captain, r.foreign
                        ],
                    )?;
                }
            }
        }
        conn.pragma_update(None, "user_version", 3)?;
        Ok(())
    })();
    match result {
        Ok(()) => {
            conn.execute_batch("COMMIT;")?;
            Ok(())
        }
        Err(e) => {
            let _ = conn.execute_batch("ROLLBACK;");
            Err(e)
        }
    }
}

/// Version 4: season settings, and the tables each match coded here was
/// created with.
fn migrate_3_to_4(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "BEGIN;
         ALTER TABLE match ADD COLUMN tables TEXT;
         CREATE TABLE setting (key TEXT PRIMARY KEY, value TEXT NOT NULL);
         PRAGMA user_version = 4;
         COMMIT;",
    )?;
    Ok(())
}

/// Version 6: the match video's path.
fn migrate_5_to_6(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "BEGIN;
         ALTER TABLE match ADD COLUMN video_path TEXT;
         PRAGMA user_version = 6;
         COMMIT;",
    )?;
    Ok(())
}

/// Version 7: where each play went on the court.
fn migrate_6_to_7(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "BEGIN;
         ALTER TABLE event ADD COLUMN start_xy INTEGER;
         ALTER TABLE event ADD COLUMN mid_xy INTEGER;
         ALTER TABLE event ADD COLUMN end_xy INTEGER;
         PRAGMA user_version = 7;
         COMMIT;",
    )?;
    Ok(())
}

/// Version 8: a set end's length in minutes as the scout gave it (#21).
fn migrate_7_to_8(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "BEGIN;
         ALTER TABLE event ADD COLUMN minutes INTEGER;
         PRAGMA user_version = 8;
         COMMIT;",
    )?;
    Ok(())
}

/// Version 9: each team's colour (v6 T13).
fn migrate_8_to_9(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "BEGIN;
         ALTER TABLE team ADD COLUMN colour TEXT NOT NULL DEFAULT '';
         PRAGMA user_version = 9;
         COMMIT;",
    )?;
    Ok(())
}

/// Version 5 lets event rows mark a golden set (kind `golden`); the table
/// is rebuilt for the new CHECK constraint, as in version 2.
fn migrate_4_to_5(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "BEGIN;
         CREATE TABLE event_v5 (
             match_id TEXT NOT NULL REFERENCES match(id),
             seq INTEGER NOT NULL,
             id TEXT NOT NULL,
             kind TEXT NOT NULL CHECK (kind IN ('code', 'comment', 'removed', 'six', 'golden')),
             text TEXT NOT NULL,
             video_ms INTEGER,
             clock TEXT,
             producer TEXT NOT NULL,
             supersedes TEXT,
             before TEXT,
             source_line INTEGER,
             PRIMARY KEY (match_id, seq)
         );
         INSERT INTO event_v5 SELECT * FROM event;
         DROP TABLE event;
         ALTER TABLE event_v5 RENAME TO event;
         PRAGMA user_version = 5;
         COMMIT;",
    )?;
    Ok(())
}

pub(crate) fn role_number(r: dvw_codec::dvw::Role) -> u8 {
    use dvw_codec::dvw::Role;
    match r {
        Role::Libero => 1,
        Role::Outside => 2,
        Role::Opposite => 3,
        Role::Middle => 4,
        Role::Setter => 5,
        Role::Unknown => 6,
    }
}

/// A new team or match id: `now_ms` and the season's counter.
fn next_id(conn: &Connection, now_ms: u64) -> Result<Ulid> {
    let n: i64 = conn.query_row(
        "UPDATE season SET next_id = next_id + 1 RETURNING next_id - 1",
        [],
        |r| r.get(0),
    )?;
    Ok(Ulid::from_parts(now_ms, u128::try_from(n).unwrap_or(0)))
}

fn ms(now_ms: u64) -> i64 {
    i64::try_from(now_ms).unwrap_or(i64::MAX)
}

fn write_events(conn: &Connection, id: MatchId, events: &[MatchEvent], from: usize) -> Result<()> {
    let mut stmt = conn.prepare(
        "INSERT INTO event (match_id, seq, id, kind, text, video_ms, clock, producer, supersedes, before, source_line, start_xy, mid_xy, end_xy, minutes)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
    )?;
    for (i, e) in events.iter().enumerate() {
        let (kind, text) = match &e.body {
            Body::Code(c) => ("code", c.to_string()),
            Body::Comment(t) => ("comment", t.clone()),
            Body::Removed => ("removed", String::new()),
            Body::StartingSix { team, court } => ("six", six_text(*team, court)),
            Body::GoldenSet => ("golden", String::new()),
        };
        let seq = i64::try_from(from + i).unwrap_or(i64::MAX);
        stmt.execute(params![
            id.to_string(),
            seq,
            e.id.to_string(),
            kind,
            text,
            e.video_ms.map(ms),
            e.clock
                .map(|c| format!("{:02}:{:02}:{:02}", c.hour, c.minute, c.second)),
            producer_name(e.producer),
            e.supersedes.map(|s| s.to_string()),
            e.before.map(|s| s.to_string()),
            e.source_line.map(|l| i64::try_from(l).unwrap_or(i64::MAX)),
            e.points.start,
            e.points.mid,
            e.points.end,
            e.minutes,
        ])?;
    }
    Ok(())
}

type EventRow = (
    u32,
    String,
    String,
    String,
    Option<i64>,
    Option<String>,
    String,
    Option<String>,
    Option<String>,
    Option<i64>,
    Points,
    Option<u16>,
);

fn read_events(conn: &Connection, id: MatchId) -> Result<Vec<MatchEvent>> {
    let mut stmt = conn.prepare(
        "SELECT seq, id, kind, text, video_ms, clock, producer, supersedes, before, source_line, start_xy, mid_xy, end_xy, minutes
         FROM event WHERE match_id = ?1 ORDER BY seq",
    )?;
    let rows = stmt.query_map([id.to_string()], |r| {
        Ok((
            r.get(0)?,
            r.get(1)?,
            r.get(2)?,
            r.get(3)?,
            r.get(4)?,
            r.get(5)?,
            r.get(6)?,
            r.get(7)?,
            r.get(8)?,
            r.get(9)?,
            Points {
                start: r.get(10)?,
                mid: r.get(11)?,
                end: r.get(12)?,
            },
            r.get(13)?,
        ))
    })?;
    rows.map(|row| {
        let (
            seq,
            eid,
            kind,
            text,
            video_ms,
            clock,
            producer,
            supersedes,
            before,
            source_line,
            points,
            minutes,
        ): EventRow = row?;
        let body = match kind.as_str() {
            "code" => Body::Code(Code::parse(&text)),
            "comment" => Body::Comment(text),
            "removed" => Body::Removed,
            "six" => parse_six(&text)?,
            "golden" => Body::GoldenSet,
            other => return Err(Error::BadRow(format!("event kind {other:?}"))),
        };
        Ok(MatchEvent {
            id: event_id(&eid)?,
            seq,
            body,
            video_ms: video_ms.and_then(|v| u64::try_from(v).ok()),
            clock: clock.as_deref().map(parse_clock).transpose()?,
            producer: parse_producer(&producer)?,
            supersedes: supersedes.as_deref().map(event_id).transpose()?,
            before: before.as_deref().map(event_id).transpose()?,
            source_line: source_line.and_then(|l| usize::try_from(l).ok()),
            points,
            minutes,
        })
    })
    .collect()
}

fn six_text(team: Side, court: &[u8; 6]) -> String {
    let mut s = team.as_char().to_string();
    for n in court {
        s.push(' ');
        s.push_str(&n.to_string());
    }
    s
}

fn parse_six(text: &str) -> Result<Body> {
    let bad = || Error::BadRow(format!("starting six {text:?}"));
    let mut parts = text.split(' ');
    let team = match parts.next() {
        Some("*") => Side::Home,
        Some("a") => Side::Visiting,
        _ => return Err(bad()),
    };
    let numbers: Vec<u8> = parts
        .map(|p| p.parse().map_err(|_| bad()))
        .collect::<Result<_>>()?;
    let court: [u8; 6] = numbers.try_into().map_err(|_| bad())?;
    Ok(Body::StartingSix { team, court })
}

fn event_id(s: &str) -> Result<EventId> {
    Ulid::from_string(s)
        .map(EventId)
        .map_err(|_| Error::BadRow(format!("event id {s:?}")))
}

fn parse_clock(s: &str) -> Result<ClockTime> {
    let bad = || Error::BadRow(format!("clock {s:?}"));
    let mut parts = s.split(':').map(|p| p.parse::<u8>().map_err(|_| bad()));
    let (Some(hour), Some(minute), Some(second), None) =
        (parts.next(), parts.next(), parts.next(), parts.next())
    else {
        return Err(bad());
    };
    Ok(ClockTime {
        hour: hour?,
        minute: minute?,
        second: second?,
    })
}

fn producer_name(p: Producer) -> &'static str {
    match p {
        Producer::Keyboard => "keyboard",
        Producer::Tap => "tap",
        Producer::Import(ImportFormat::Dvw) => "import:dvw",
        Producer::Import(ImportFormat::Vsm) => "import:vsm",
        Producer::Repair => "repair",
    }
}

fn parse_producer(s: &str) -> Result<Producer> {
    Ok(match s {
        "keyboard" => Producer::Keyboard,
        "tap" => Producer::Tap,
        "import:dvw" => Producer::Import(ImportFormat::Dvw),
        "import:vsm" => Producer::Import(ImportFormat::Vsm),
        "repair" => Producer::Repair,
        other => return Err(Error::BadRow(format!("producer {other:?}"))),
    })
}
