//! The coding console's session (docs/adr/0001): the open season and match,
//! held in the desktop backend. The UI sends codes and commands and gets a
//! [`Snapshot`] back; every change is saved before the snapshot is built.
//!
//! Nothing here knows about Tauri, so the whole loop is tested headless.

pub mod manage;
pub mod readback;
pub mod resume;
pub mod snapshot;

use std::path::Path;

use courtside_store::Season;
use vb_engine::log::{EventLog, LogError};
use vb_engine::model::Outcome;
use vb_engine::{Body, EventId, IdGen, Match, MatchEvent, NewEvent, Producer, fold};
use vb_grammar::Code;

pub use courtside_store::{Error as StoreError, MatchId};
pub use manage::{PlayerDetail, PlayerListItem, TeamDetail, TeamForm};
pub use readback::Preview;
pub use resume::Resume;
pub use snapshot::{MatchItem, Snapshot};
pub use vb_engine::event::ClockTime;
pub use vb_grammar::Team;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Store(#[from] courtside_store::Error),
    #[error("no season is open")]
    NoSeason,
    #[error("no match is open")]
    NoMatch,
    #[error("{0}")]
    BadCode(String),
    #[error(transparent)]
    Log(#[from] LogError),
    #[error("lineup: {0}")]
    BadLineup(String),
    #[error("can't write {path}: {source}")]
    Write {
        path: String,
        source: std::io::Error,
    },
    #[error("can't read {path}: {source}")]
    Read {
        path: String,
        source: std::io::Error,
    },
}

pub type Result<T> = std::result::Result<T, Error>;

/// The match being coded.
struct Current {
    id: MatchId,
    game: Match,
    teams: (courtside_store::Team, courtside_store::Team),
    /// New event ids; the time part is when the match was opened.
    ids: IdGen,
    /// What undo took off, newest last, for redo. Any other change
    /// clears it. Kept in memory only.
    redo: Vec<Vec<MatchEvent>>,
}

/// Whether `score` wins `set` under the match's regulation.
fn won(game: &Match, set: u8, score: vb_engine::Score) -> bool {
    let target = game.regulation.set_target(set);
    let (hi, lo) = (
        score.home.max(score.visiting),
        score.home.min(score.visiting),
    );
    hi >= target && hi - lo >= 2
}

fn other(t: Team) -> Team {
    match t {
        Team::Home => Team::Visiting,
        Team::Visiting => Team::Home,
    }
}

impl Current {
    fn names(&self) -> (&str, &str) {
        (&self.teams.0.name, &self.teams.1.name)
    }

    /// Saves the log after a change that added the events from `before`
    /// on; if saving fails, takes them off again. A change clears redo.
    fn persist(&mut self, season: &mut Season, before: usize) -> Result<()> {
        if let Err(e) = season.save(self.id, &self.game.log) {
            while self.game.log.len() > before {
                self.game.log.undo();
            }
            return Err(e.into());
        }
        self.redo.clear();
        Ok(())
    }

    fn add(&mut self, code: Code, producer: Producer, clock: Option<ClockTime>) {
        let event = NewEvent {
            body: Body::Code(code),
            video_ms: None,
            clock,
            producer,
        };
        self.game.log.append(&mut self.ids, event);
    }
}

/// One scout's console: at most one season and one match open.
#[derive(Default)]
pub struct Session {
    season: Option<Season>,
    season_path: Option<std::path::PathBuf>,
    current: Option<Current>,
}

impl Session {
    pub fn new() -> Self {
        Session::default()
    }

    /// Creates a season database at `path` and opens it.
    pub fn create_season(&mut self, path: &Path, name: &str, now_ms: u64) -> Result<()> {
        self.season = Some(Season::create(path, name, now_ms)?);
        self.season_path = Some(path.to_owned());
        self.current = None;
        Ok(())
    }

    pub fn open_season(&mut self, path: &Path) -> Result<()> {
        self.season = Some(Season::open(path)?);
        self.season_path = Some(path.to_owned());
        self.current = None;
        Ok(())
    }

    pub fn matches(&self) -> Result<Vec<MatchItem>> {
        Ok(self
            .season()?
            .matches()?
            .into_iter()
            .map(MatchItem::from)
            .collect())
    }

    /// Every team of the season with its roster.
    pub fn teams(&self) -> Result<Vec<TeamDetail>> {
        Ok(self
            .season()?
            .teams()?
            .into_iter()
            .map(TeamDetail::from)
            .collect())
    }

    /// Creates (`id` `None`) or updates a team and its roster together.
    pub fn save_team(
        &mut self,
        id: Option<&str>,
        team: &TeamForm,
        players: &[PlayerDetail],
        now_ms: u64,
    ) -> Result<TeamDetail> {
        let rows: Vec<_> = players.iter().map(Into::into).collect();
        let season = self.season_mut()?;
        let id = season.save_team(id, &team.into(), &rows, now_ms)?;
        Ok(season.team(&id)?.into())
    }

    pub fn delete_team(&mut self, id: &str) -> Result<()> {
        Ok(self.season_mut()?.delete_team(id)?)
    }

    /// Folds team `from` into `into`; `pairs` maps `from`'s players to the
    /// `into` players they are.
    pub fn merge_teams(
        &mut self,
        from: &str,
        into: &str,
        pairs: &[(String, String)],
    ) -> Result<()> {
        Ok(self.season_mut()?.merge_teams(from, into, pairs)?)
    }

    pub fn players(&self) -> Result<Vec<PlayerListItem>> {
        Ok(self
            .season()?
            .players()?
            .into_iter()
            .map(PlayerListItem::from)
            .collect())
    }

    pub fn merge_players(&mut self, keep: &str, drop: &str) -> Result<()> {
        Ok(self.season_mut()?.merge_players(keep, drop)?)
    }

    /// The open match's roster for one side, for editing.
    pub fn match_roster(&self, home: bool) -> Result<Vec<PlayerDetail>> {
        let c = self.current.as_ref().ok_or(Error::NoMatch)?;
        let (h, v) = self.season()?.match_roster(c.id)?;
        Ok(if home { h } else { v }
            .into_iter()
            .map(PlayerDetail::from)
            .collect())
    }

    /// Replaces one side's roster for the open match only, and reopens it.
    pub fn save_match_roster(
        &mut self,
        home: bool,
        players: &[PlayerDetail],
        now_ms: u64,
    ) -> Result<Snapshot> {
        let id = self.current.as_ref().ok_or(Error::NoMatch)?.id;
        let rows: Vec<_> = players.iter().map(Into::into).collect();
        self.season_mut()?.save_match_roster(id, home, &rows)?;
        self.open_match(id, now_ms)
    }

    /// Creates a match between two of the season's teams (by id), to be
    /// coded here, and opens it. `date` is ISO `YYYY-MM-DD`.
    pub fn new_match(
        &mut self,
        home_team_id: &str,
        visiting_team_id: &str,
        date: Option<&str>,
        now_ms: u64,
    ) -> Result<Snapshot> {
        let id = self
            .season_mut()?
            .create_match(home_team_id, visiting_team_id, date, now_ms)?;
        self.open_match(id, now_ms)
    }

    /// Imports a `.dvw` file into the open season (without opening it).
    pub fn import_file(&mut self, path: &Path, now_ms: u64) -> Result<MatchId> {
        let bytes = std::fs::read(path).map_err(|source| Error::Read {
            path: path.display().to_string(),
            source,
        })?;
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        Ok(self.season_mut()?.import_dvw(&name, &bytes, now_ms)?)
    }

    /// Opens a match of the season for coding.
    pub fn open_match(&mut self, id: MatchId, now_ms: u64) -> Result<Snapshot> {
        let season = self.season()?;
        let open = season.open_match(id)?;
        let teams = season.match_teams(id)?;
        self.current = Some(Current {
            id,
            game: open.game,
            teams,
            ids: IdGen::new(now_ms),
            redo: Vec::new(),
        });
        self.snapshot()
    }

    /// Where the scout is, to reopen after a restart.
    pub fn resume_point(&self) -> Resume {
        Resume {
            season: self.season_path.clone(),
            match_id: self.current.as_ref().map(|c| c.id.to_string()),
        }
    }

    /// Reopens what [`Session::resume_point`] recorded. The season opens
    /// even if its match no longer does.
    pub fn resume(&mut self, at: &Resume, now_ms: u64) -> Result<Option<Snapshot>> {
        let Some(path) = &at.season else {
            return Ok(None);
        };
        self.open_season(path)?;
        let Some(id) = at.match_id.as_deref().and_then(|m| m.parse().ok()) else {
            return Ok(None);
        };
        Ok(self.open_match(id, now_ms).ok())
    }

    /// The open match's state, for the UI.
    pub fn snapshot(&self) -> Result<Snapshot> {
        let c = self.current.as_ref().ok_or(Error::NoMatch)?;
        Ok(snapshot::build(&c.id.to_string(), &c.game, &c.teams))
    }

    /// Reads back what the command line holds, without changing anything.
    pub fn preview(&self, text: &str) -> Result<Preview> {
        let c = self.current.as_ref().ok_or(Error::NoMatch)?;
        Ok(readback::preview(text, &c.game, c.names()))
    }

    /// Adds the codes on the command line (separated by spaces) to the
    /// match, then what follows from them: the point code when a touch
    /// decides the rally, and the winner's rotation on a side-out. All of
    /// it is saved before the new state is returned; if saving fails the
    /// match is left as it was.
    pub fn commit(&mut self, text: &str, clock: Option<ClockTime>) -> Result<Snapshot> {
        let c = self.current.as_mut().ok_or(Error::NoMatch)?;
        let season = self.season.as_mut().ok_or(Error::NoSeason)?;
        let preview = readback::preview(text, &c.game, c.names());
        if !preview.ok {
            let why = preview
                .codes
                .iter()
                .find_map(|r| r.error.as_ref().map(|e| format!("{}: {e}", r.text)))
                .unwrap_or_else(|| "Type a code first.".into());
            return Err(Error::BadCode(why));
        }
        let before = c.game.log.len();
        let codes: Vec<Code> = text.split_whitespace().map(Code::parse).collect();
        let typed_point = codes.iter().rev().find_map(|code| match code {
            Code::Point { team, .. } => Some(*team),
            _ => None,
        });
        let decided = codes.iter().rev().find_map(|code| match code {
            Code::Touch(t) => {
                let o = c.game.winning_symbols.outcome(t.skill, t.evaluation()?)?;
                Some(if o == Outcome::Win {
                    t.team
                } else {
                    other(t.team)
                })
            }
            _ => None,
        });
        for code in codes {
            c.add(code, Producer::Keyboard, clock);
        }
        if let Some(winner) = typed_point.or(decided) {
            let folded = fold(&c.game);
            let serving = folded.rallies.last().and_then(|r| r.serving);
            let last = folded.states.last();
            if typed_point.is_none() {
                let score = last.map(|s| s.score).unwrap_or_default();
                let (h, v) = match winner {
                    Team::Home => (score.home.saturating_add(1), score.visiting),
                    Team::Visiting => (score.home, score.visiting.saturating_add(1)),
                };
                let point = format!("{}p{h:02}:{v:02}", winner.as_char());
                c.add(Code::parse(&point), Producer::Repair, clock);
            }
            let after = fold(&c.game);
            let end = after.states.last();
            let set = end.map_or(1, |s| s.set);
            let score = end.map(|s| s.score).unwrap_or_default();
            if won(&c.game, set, score) {
                c.add(Code::parse(&format!("**{set}set")), Producer::Repair, clock);
            } else {
                // Side-out: the winner rotates, its setter one zone on.
                let zone = last.and_then(|s| *s.setter_position.get(winner));
                if let (true, Some(z)) = (serving.is_some_and(|s| s != winner), zone) {
                    let next = if z == 1 { 6 } else { z - 1 };
                    let rotation = format!("{}z{next}", winner.as_char());
                    c.add(Code::parse(&rotation), Producer::Repair, clock);
                }
            }
        }
        c.persist(season, before)?;
        self.snapshot()
    }

    /// Enters `team`'s six for the set in progress (zones 1–6) and its
    /// setter, written as DataVolley does: the six as starting zones, then
    /// `P` and `z` codes marked `>LUp`.
    pub fn set_lineup(
        &mut self,
        team: Team,
        court: [u8; 6],
        setter: u8,
        clock: Option<ClockTime>,
    ) -> Result<Snapshot> {
        let c = self.current.as_mut().ok_or(Error::NoMatch)?;
        let season = self.season.as_mut().ok_or(Error::NoSeason)?;
        let sheet = c.game.teams.get(team);
        for (i, n) in court.iter().enumerate() {
            if court[..i].contains(n) {
                return Err(Error::BadLineup(format!("#{n} is in two zones")));
            }
            match sheet.player(*n) {
                None => return Err(Error::BadLineup(format!("#{n} is not on the team sheet"))),
                Some(p) if p.libero => {
                    return Err(Error::BadLineup(format!("#{n} is a libero")));
                }
                Some(_) => {}
            }
        }
        let zone = court
            .iter()
            .position(|&n| n == setter)
            .ok_or_else(|| Error::BadLineup(format!("setter #{setter} is not in the six")))?
            + 1;
        let before = c.game.log.len();
        c.game.log.append(
            &mut c.ids,
            NewEvent {
                body: Body::StartingSix { team, court },
                video_ms: None,
                clock,
                producer: Producer::Keyboard,
            },
        );
        let t = team.as_char();
        c.add(
            Code::parse(&format!("{t}P{setter:02}>LUp")),
            Producer::Repair,
            clock,
        );
        c.add(
            Code::parse(&format!("{t}z{zone}>LUp")),
            Producer::Repair,
            clock,
        );
        c.persist(season, before)?;
        self.snapshot()
    }

    /// Takes back the scout's last action: the codes the console wrote
    /// after it (point, rotation, set end, lineup codes) and the action
    /// itself, one code or the six. Nothing to undo is not an error.
    pub fn undo(&mut self) -> Result<Snapshot> {
        let c = self.current.as_mut().ok_or(Error::NoMatch)?;
        let season = self.season.as_mut().ok_or(Error::NoSeason)?;
        let mut taken = Vec::new();
        while let Some(e) = c.game.log.undo() {
            let auto = e.producer == Producer::Repair;
            taken.push(e);
            if !auto {
                break;
            }
        }
        if taken.is_empty() {
            return self.snapshot();
        }
        taken.reverse();
        if let Err(e) = season.save(c.id, &c.game.log) {
            for event in taken {
                // Puts back exactly what was taken off, so it fits.
                let _ = c.game.log.redo(event);
            }
            return Err(e.into());
        }
        c.redo.push(taken);
        self.snapshot()
    }

    /// Puts back what the last undo took off.
    pub fn redo(&mut self) -> Result<Snapshot> {
        let c = self.current.as_mut().ok_or(Error::NoMatch)?;
        let season = self.season.as_mut().ok_or(Error::NoSeason)?;
        let Some(events) = c.redo.pop() else {
            return self.snapshot();
        };
        let before = c.game.log.len();
        for event in events.iter().cloned() {
            if let Err(e) = c.game.log.redo(event) {
                while c.game.log.len() > before {
                    c.game.log.undo();
                }
                return Err(Error::Log(e));
            }
        }
        if let Err(e) = season.save(c.id, &c.game.log) {
            while c.game.log.len() > before {
                c.game.log.undo();
            }
            c.redo.push(events);
            return Err(e.into());
        }
        self.snapshot()
    }

    /// Replaces an earlier code with `text` (one code), in place.
    pub fn correct(&mut self, id: &str, text: &str, clock: Option<ClockTime>) -> Result<Snapshot> {
        self.edit(id, Some(text), clock, |log, ids, target, new| match new {
            Some(new) => log.correct(ids, target, new),
            None => Err(LogError::NotFound(target)),
        })
    }

    /// Adds `text` (one code) just before an earlier code.
    pub fn insert_before(
        &mut self,
        id: &str,
        text: &str,
        clock: Option<ClockTime>,
    ) -> Result<Snapshot> {
        self.edit(id, Some(text), clock, |log, ids, target, new| match new {
            Some(new) => log.insert_before(ids, target, new),
            None => Err(LogError::NotFound(target)),
        })
    }

    /// Takes an earlier code out of the match.
    pub fn remove(&mut self, id: &str) -> Result<Snapshot> {
        self.edit(id, None, None, |log, ids, target, _| {
            log.remove(ids, target, Producer::Keyboard)
        })
    }

    fn edit(
        &mut self,
        id: &str,
        text: Option<&str>,
        clock: Option<ClockTime>,
        change: impl FnOnce(
            &mut EventLog,
            &mut IdGen,
            EventId,
            Option<NewEvent>,
        ) -> std::result::Result<EventId, LogError>,
    ) -> Result<Snapshot> {
        let c = self.current.as_mut().ok_or(Error::NoMatch)?;
        let season = self.season.as_mut().ok_or(Error::NoSeason)?;
        let target = ulid::Ulid::from_string(id)
            .map(EventId)
            .map_err(|_| Error::BadCode(format!("no event {id:?}")))?;
        let new = match text {
            None => None,
            Some(text) => {
                let text = text.trim();
                if text.split_whitespace().count() != 1 {
                    return Err(Error::BadCode("One code at a time here.".into()));
                }
                let read = readback::read(text, &c.game, c.names());
                if let Some(e) = read.error {
                    return Err(Error::BadCode(format!("{text}: {e}")));
                }
                Some(NewEvent {
                    body: Body::Code(Code::parse(text)),
                    video_ms: None,
                    clock,
                    producer: Producer::Keyboard,
                })
            }
        };
        let before = c.game.log.len();
        change(&mut c.game.log, &mut c.ids, target, new)?;
        c.persist(season, before)?;
        self.snapshot()
    }

    /// Writes the open match as a `.dvw` file.
    pub fn export_dvw(&self, path: &Path) -> Result<()> {
        let c = self.current.as_ref().ok_or(Error::NoMatch)?;
        let bytes = self.season()?.export_dvw(c.id)?;
        std::fs::write(path, bytes).map_err(|source| Error::Write {
            path: path.display().to_string(),
            source,
        })
    }

    fn season(&self) -> Result<&Season> {
        self.season.as_ref().ok_or(Error::NoSeason)
    }

    fn season_mut(&mut self) -> Result<&mut Season> {
        self.season.as_mut().ok_or(Error::NoSeason)
    }
}
