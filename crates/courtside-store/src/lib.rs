//! The season database (PRD F-01, docs/adr/0001): one SQLite file per
//! season holding teams, players and matches. A match is its event log,
//! one row per [`MatchEvent`], replayed into vb-engine on open; an imported
//! match also keeps its source `.dvw` bytes, which supply the headers on
//! export.
//!
//! The log only grows at the tail or shrinks by undo, so [`Season::save`]
//! writes the difference: rows past the first event that differs are
//! replaced, in one transaction.

use std::path::Path;

use dvw_codec::Dvw;
use rusqlite::{Connection, OptionalExtension, params};
use ulid::Ulid;
use vb_engine::event::{ClockTime, ImportFormat};
use vb_engine::log::{EventLog, LogError};
use vb_engine::{Body, EventId, IdGen, Match, MatchEvent, Producer};
use vb_grammar::Code;
use vb_grammar::Team as Side;

/// Schema version, kept in SQLite's `user_version`.
const VERSION: u32 = 2;

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
    coach TEXT NOT NULL,
    assistant TEXT NOT NULL
);
CREATE TABLE player (
    team_id TEXT NOT NULL REFERENCES team(id),
    number INTEGER NOT NULL,
    last_name TEXT NOT NULL,
    first_name TEXT NOT NULL,
    libero INTEGER NOT NULL,
    PRIMARY KEY (team_id, number)
);
CREATE TABLE match (
    id TEXT PRIMARY KEY,
    home_team_id TEXT NOT NULL REFERENCES team(id),
    visiting_team_id TEXT NOT NULL REFERENCES team(id),
    date TEXT,
    source_name TEXT,
    source_dvw BLOB,
    created_ms INTEGER NOT NULL
);
CREATE TABLE event (
    match_id TEXT NOT NULL REFERENCES match(id),
    seq INTEGER NOT NULL,
    id TEXT NOT NULL,
    -- A starting six's text is the team prefix and six shirt numbers in
    -- zone order, e.g. '* 8 12 13 11 3 14'.
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
    #[error("match {0} has no source .dvw to take headers from")]
    NoTemplate(MatchId),
    #[error("bad stored value: {0}")]
    BadRow(String),
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
pub struct Team {
    pub id: String,
    pub name: String,
    pub coach: String,
    pub assistant: String,
    pub players: Vec<Player>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Player {
    pub number: u8,
    pub last_name: String,
    pub first_name: String,
    pub libero: bool,
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

/// A match ready to code: the engine's view and, for imported matches, the
/// file its headers come from.
#[derive(Debug, Clone)]
pub struct OpenMatch {
    pub id: MatchId,
    pub game: Match,
    pub template: Option<Dvw>,
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
            1 => migrate_1_to_2(&conn)?,
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
            for p in players {
                let Some(number) = p.number() else { continue };
                let libero = if i == 0 {
                    game.teams.home.is_libero(number)
                } else {
                    game.teams.visiting.is_libero(number)
                };
                tx.execute(
                    "INSERT OR IGNORE INTO player (team_id, number, last_name, first_name, libero)
                     VALUES (?1, ?2, ?3, ?4, ?5)",
                    params![team_id, number, p.last_name(), p.first_name(), libero],
                )?;
            }
            team_ids.push(team_id);
        }
        tx.execute(
            "INSERT INTO match (id, home_team_id, visiting_team_id, date, source_name, source_dvw, created_ms)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                id.to_string(),
                team_ids[0],
                team_ids[1],
                dvw.match_date().map(|(d, _)| d.to_string()),
                file_name,
                bytes,
                ms(now_ms)
            ],
        )?;
        write_events(&tx, id, game.log.all(), 0)?;
        tx.commit()?;
        Ok(id)
    }

    pub fn teams(&self) -> Result<Vec<Team>> {
        let mut teams: Vec<Team> = self
            .conn
            .prepare("SELECT id, name, coach, assistant FROM team ORDER BY name")?
            .query_map([], |r| {
                Ok(Team {
                    id: r.get(0)?,
                    name: r.get(1)?,
                    coach: r.get(2)?,
                    assistant: r.get(3)?,
                    players: Vec::new(),
                })
            })?
            .collect::<rusqlite::Result<_>>()?;
        let mut stmt = self.conn.prepare(
            "SELECT number, last_name, first_name, libero FROM player WHERE team_id = ?1 ORDER BY number",
        )?;
        for t in &mut teams {
            t.players = stmt
                .query_map([&t.id], |r| {
                    Ok(Player {
                        number: r.get(0)?,
                        last_name: r.get(1)?,
                        first_name: r.get(2)?,
                        libero: r.get(3)?,
                    })
                })?
                .collect::<rusqlite::Result<_>>()?;
        }
        Ok(teams)
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

    /// The match's home and visiting teams, with their rosters.
    pub fn match_teams(&self, id: MatchId) -> Result<(Team, Team)> {
        let ids: Option<(String, String)> = self
            .conn
            .query_row(
                "SELECT home_team_id, visiting_team_id FROM match WHERE id = ?1",
                [id.to_string()],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()?;
        let (home_id, visiting_id) = ids.ok_or(Error::NoMatch(id))?;
        let teams = self.teams()?;
        let find = |team_id: &str| {
            teams
                .iter()
                .find(|t| t.id == team_id)
                .cloned()
                .ok_or_else(|| Error::BadRow(format!("team {team_id:?}")))
        };
        Ok((find(&home_id)?, find(&visiting_id)?))
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
        let Some(bytes) = source else {
            return Err(Error::NoTemplate(id));
        };
        let template = Dvw::parse(&bytes)?;
        let mut game = Match::from_dvw(&template, &mut IdGen::new(0));
        game.log = EventLog::from_events(read_events(&self.conn, id)?)?;
        Ok(OpenMatch {
            id,
            game,
            template: Some(template),
        })
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

    /// The match as `.dvw` bytes: headers from its source file, everything
    /// the engine derives regenerated.
    pub fn export_dvw(&self, id: MatchId) -> Result<Vec<u8>> {
        let open = self.open_match(id)?;
        let template = open.template.ok_or(Error::NoTemplate(id))?;
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
        "INSERT INTO event (match_id, seq, id, kind, text, video_ms, clock, producer, supersedes, before, source_line)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
    )?;
    for (i, e) in events.iter().enumerate() {
        let (kind, text) = match &e.body {
            Body::Code(c) => ("code", c.to_string()),
            Body::Comment(t) => ("comment", t.clone()),
            Body::Removed => ("removed", String::new()),
            Body::StartingSix { team, court } => ("six", six_text(*team, court)),
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
);

fn read_events(conn: &Connection, id: MatchId) -> Result<Vec<MatchEvent>> {
    let mut stmt = conn.prepare(
        "SELECT seq, id, kind, text, video_ms, clock, producer, supersedes, before, source_line
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
        ))
    })?;
    rows.map(|row| {
        let (seq, eid, kind, text, video_ms, clock, producer, supersedes, before, source_line): EventRow =
            row?;
        let body = match kind.as_str() {
            "code" => Body::Code(Code::parse(&text)),
            "comment" => Body::Comment(text),
            "removed" => Body::Removed,
            "six" => parse_six(&text)?,
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
