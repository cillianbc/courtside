//! The coding console's session (docs/adr/0001): the open season and match,
//! held in the desktop backend. The UI sends codes and commands and gets a
//! [`Snapshot`] back; every change is saved before the snapshot is built.
//!
//! Nothing here knows about Tauri, so the whole loop is tested headless.

pub mod readback;
pub mod snapshot;

use std::path::Path;

use courtside_store::Season;
use vb_engine::model::Outcome;
use vb_engine::{Body, IdGen, Match, NewEvent, Producer, fold};
use vb_grammar::{Code, Team};

pub use courtside_store::{Error as StoreError, MatchId};
pub use readback::Preview;
pub use snapshot::{MatchItem, Snapshot};
pub use vb_engine::event::ClockTime;

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
    current: Option<Current>,
}

impl Session {
    pub fn new() -> Self {
        Session::default()
    }

    /// Creates a season database at `path` and opens it.
    pub fn create_season(&mut self, path: &Path, name: &str, now_ms: u64) -> Result<()> {
        self.season = Some(Season::create(path, name, now_ms)?);
        self.current = None;
        Ok(())
    }

    pub fn open_season(&mut self, path: &Path) -> Result<()> {
        self.season = Some(Season::open(path)?);
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
        });
        self.snapshot()
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
            // Side-out: the winner rotates, its setter one zone on.
            let zone = last.and_then(|s| *s.setter_position.get(winner));
            if let (true, Some(z)) = (serving.is_some_and(|s| s != winner), zone) {
                let next = if z == 1 { 6 } else { z - 1 };
                let rotation = format!("{}z{next}", winner.as_char());
                c.add(Code::parse(&rotation), Producer::Repair, clock);
            }
        }
        if let Err(e) = season.save(c.id, &c.game.log) {
            while c.game.log.len() > before {
                c.game.log.undo();
            }
            return Err(e.into());
        }
        self.snapshot()
    }

    fn season(&self) -> Result<&Season> {
        self.season.as_ref().ok_or(Error::NoSeason)
    }

    fn season_mut(&mut self) -> Result<&mut Season> {
        self.season.as_mut().ok_or(Error::NoSeason)
    }
}
