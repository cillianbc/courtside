//! The coding console's session (docs/adr/0001): the open season and match,
//! held in the desktop backend. The UI sends codes and commands and gets a
//! [`Snapshot`] back; every change is saved before the snapshot is built.
//!
//! Nothing here knows about Tauri, so the whole loop is tested headless.

pub mod snapshot;

use std::path::Path;

use courtside_store::{Season, Team};
use vb_engine::{IdGen, Match};

pub use courtside_store::{Error as StoreError, MatchId};
pub use snapshot::{MatchItem, Snapshot};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Store(#[from] courtside_store::Error),
    #[error("no season is open")]
    NoSeason,
    #[error("no match is open")]
    NoMatch,
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
    teams: (Team, Team),
    /// New event ids; the time part is when the match was opened.
    #[allow(dead_code)] // used once codes can be committed (step C.2)
    ids: IdGen,
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

    fn season(&self) -> Result<&Season> {
        self.season.as_ref().ok_or(Error::NoSeason)
    }

    fn season_mut(&mut self) -> Result<&mut Season> {
        self.season.as_mut().ok_or(Error::NoSeason)
    }
}
