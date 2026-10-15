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
use vb_engine::{
    Body, EventId, EventKind, IdGen, Match, MatchEvent, NewEvent, Producer, TeamSheet, fold,
};
use vb_grammar::expand::{Combination, Defaults, follow_combination, follow_pair, is_pair};
use vb_grammar::{Code, Skill, Touch};

pub use courtside_store::{Error as StoreError, MatchId, SKILLS, Settings};
pub use manage::{PlayerDetail, PlayerListItem, TeamDetail, TeamForm, TeamImportView};
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
    /// Typing defaults (season settings, step E).
    defaults: Defaults,
    /// Hold set ends until the scout confirms (season settings).
    confirm_set_end: bool,
}

/// Whether `score` wins `set` under the match's regulation (a golden set
/// goes to 15).
fn won(game: &Match, folded: &vb_engine::Folded, set: u8, score: vb_engine::Score) -> bool {
    let target = folded.set_target(game, set);
    let (hi, lo) = (
        score.home.max(score.visiting),
        score.home.min(score.visiting),
    );
    hi >= target && hi - lo >= 2
}

/// The command line's defaults: typing from the season settings, attack
/// combinations from the match's own table (`combinations`).
fn defaults_from(s: &Settings, combinations: &[Combination]) -> Defaults {
    let skill = |c: char| Skill::from_char(c).unwrap_or(Skill::Attack);
    Defaults {
        skill: skill(s.typing.skill),
        types: s.typing.types,
        evaluations: s.typing.evaluations,
        predict: s.typing.predict,
        combinations: combinations.to_vec(),
    }
}

/// A match's attack combinations as expansion takes them: the imported file's `[3ATTACKCOMBINATION]`, or for a match
/// coded here the table frozen into its blank template when it was created.
/// Like DataVolley, a match uses its own table, not the season's.
fn match_combinations(template: &dvw_codec::Dvw) -> Vec<Combination> {
    template
        .attack_combinations
        .iter()
        .filter_map(|c| {
            Some(Combination {
                code: c.code().to_owned(),
                tempo: c.tempo().chars().next()?,
                start_zone: c
                    .start_zone()
                    .and_then(|z| char::from_digit(u32::from(z), 10))
                    .unwrap_or('~'),
                target: c.target_attacker().chars().next(),
            })
        })
        .collect()
}

/// Checks a six for `sheet`'s team; the setter's zone.
fn check_six(sheet: &TeamSheet, court: &[u8; 6], setter: u8) -> Result<u8> {
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
        .ok_or_else(|| Error::BadLineup(format!("setter #{setter} is not in the six")))?;
    Ok(u8::try_from(zone + 1).unwrap_or(1))
}

/// Settings the scout can't save: what a `.dvw` couldn't hold.
/// The season's rule options onto the open match.
fn apply_rules(c: &mut Current, s: &Settings) {
    c.game.rules = vb_engine::Rules {
        substitutions: s.rules.substitutions,
        libero_may_serve: s.rules.libero_may_serve,
    };
    c.confirm_set_end = s.rules.confirm_set_end;
}

fn check_settings(s: &Settings) -> Result<()> {
    let bad = |m: String| Err(Error::BadCode(m));
    if !(1..=20).contains(&s.rules.substitutions) {
        return bad("Substitutions per set: 1 to 20.".into());
    }
    if Skill::from_char(s.typing.skill).is_none() {
        return bad(format!("{} is not a skill letter", s.typing.skill));
    }
    for (i, t) in s.typing.types.iter().enumerate() {
        if !"HMQTUNO".contains(*t) {
            return bad(format!("{t} is not a type letter ({})", SKILLS[i]));
        }
    }
    for (i, e) in s.typing.evaluations.iter().enumerate() {
        if !"=/-!+#".contains(*e) {
            return bad(format!("{e} is not an evaluation ({})", SKILLS[i]));
        }
    }
    let mut codes = std::collections::HashSet::new();
    for c in &s.tables.combinations {
        let ok = c.code.chars().count() == 2
            && c.code.chars().all(|x| x.is_ascii_alphanumeric())
            && !c.code.starts_with('K');
        if !ok {
            return bad(format!(
                "combination {:?}: two letters or digits, not starting with K",
                c.code
            ));
        }
        if !codes.insert(c.code.clone()) {
            return bad(format!("combination {} is listed twice", c.code));
        }
        if !(1..=9).contains(&c.start_zone) {
            return bad(format!("combination {}: start zone 1–9", c.code));
        }
        if !"HMQTUNO".contains(c.tempo) {
            return bad(format!(
                "combination {}: {} is not a type letter",
                c.code, c.tempo
            ));
        }
    }
    for k in &s.tables.setter_calls {
        if k.code.chars().count() != 2 || !k.code.starts_with('K') || !codes.insert(k.code.clone())
        {
            return bad(format!(
                "setter call {:?}: two characters starting with K, once",
                k.code
            ));
        }
    }
    for w in &s.tables.winning {
        if !w
            .lose
            .chars()
            .chain(w.win.chars())
            .all(|c| "=/-!+#".contains(c))
        {
            return bad(format!("winning symbols for {}: evaluations only", w.skill));
        }
        if w.lose.chars().any(|c| w.win.contains(c)) {
            return bad(format!(
                "winning symbols for {}: a symbol can't both win and lose",
                w.skill
            ));
        }
    }
    Ok(())
}

fn touch_of(e: &MatchEvent) -> Option<Touch> {
    match e.code() {
        Some(Code::Touch(t)) => Some(t.clone()),
        _ => None,
    }
}

/// The visible event `target` (or what now stands in its place) forms a
/// pair with, and whether `target` is the pair's first half.
fn pair_partner(log: &EventLog, target: EventId) -> Option<(EventId, bool)> {
    let visible = log.visible();
    let i = visible.iter().position(|e| {
        let mut cur = Some(e.id);
        while let Some(id) = cur {
            if id == target {
                return true;
            }
            cur = log.get(id).and_then(|x| x.supersedes);
        }
        false
    })?;
    let me = touch_of(visible[i])?;
    if let Some(prev) = i.checked_sub(1).and_then(|j| visible.get(j))
        && let Some(p) = touch_of(prev)
        && is_pair(&p, &me)
    {
        return Some((prev.id, false));
    }
    let next = visible.get(i + 1)?;
    let n = touch_of(next)?;
    is_pair(&me, &n).then_some((next.id, true))
}

/// The touch straight before `target` (or the code it corrected) among the
/// visible events, if the event before it is a touch.
fn touch_just_before(log: &EventLog, target: EventId) -> Option<(EventId, Touch)> {
    let visible = log.visible();
    let i = visible.iter().position(|e| {
        let mut cur = Some(e.id);
        while let Some(id) = cur {
            if id == target {
                return true;
            }
            cur = log.get(id).and_then(|x| x.supersedes);
        }
        false
    })?;
    let prev = visible.get(i.checked_sub(1)?)?;
    Some((prev.id, touch_of(prev)?))
}

/// The first touch walking back through `events` before a point code or
/// set end.
fn last_touch<'a>(events: impl Iterator<Item = &'a &'a MatchEvent>) -> Option<Touch> {
    for e in events {
        match e.code() {
            Some(Code::Touch(t)) => return Some(t.clone()),
            Some(Code::Point { .. } | Code::SetEnd { .. }) => return None,
            _ => {}
        }
    }
    None
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

    fn preview(&self, text: &str) -> Preview {
        let before = self.open_rally_touch();
        readback::preview(
            text,
            &self.game,
            self.names(),
            before.as_ref(),
            &self.defaults,
        )
    }

    /// The last touch of the rally in progress (none after a point code,
    /// set end or at the start).
    fn open_rally_touch(&self) -> Option<Touch> {
        last_touch(self.game.log.visible().iter().rev())
    }

    /// The touch just before `target` in the same rally, as context for
    /// correcting or inserting there.
    fn touch_before(&self, target: EventId) -> Option<Touch> {
        let visible = self.game.log.visible();
        let root = visible.iter().position(|e| {
            e.id == target || self.game.log.get(e.id).and_then(|x| x.supersedes) == Some(target)
        })?;
        last_touch(visible[..root].iter().rev())
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

    /// The season's settings (defaults when none are saved).
    pub fn settings(&self) -> Result<Settings> {
        Ok(self.season()?.settings()?)
    }

    /// Saves the season's settings; the open match types with them at
    /// once. Tables reach matches created from now on; the open match keeps
    /// its own combinations.
    pub fn save_settings(&mut self, settings: &Settings) -> Result<()> {
        check_settings(settings)?;
        self.season_mut()?.save_settings(settings)?;
        if let Some(c) = self.current.as_mut() {
            c.defaults = defaults_from(settings, &c.defaults.combinations);
            apply_rules(c, settings);
        }
        Ok(())
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

    /// Imports a team file (`.sq` or `.vst`) into the open season.
    pub fn import_team_file(&mut self, path: &Path, now_ms: u64) -> Result<TeamImportView> {
        let bytes = std::fs::read(path).map_err(|source| Error::Read {
            path: path.display().to_string(),
            source,
        })?;
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        Ok(self
            .season_mut()?
            .import_team_file(&name, &bytes, now_ms)?
            .into())
    }

    /// Writes a team file: `.vst` when `path` ends so, else `.sq`.
    pub fn export_team_file(&self, id: &str, path: &Path) -> Result<()> {
        let vst = path
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("vst"));
        let bytes = self.season()?.export_team_file(id, vst)?;
        std::fs::write(path, bytes).map_err(|source| Error::Write {
            path: path.display().to_string(),
            source,
        })
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
        let combinations = match_combinations(&open.template);
        let settings = season.settings()?;
        let mut c = Current {
            id,
            game: open.game,
            teams,
            ids: IdGen::new(now_ms),
            redo: Vec::new(),
            defaults: defaults_from(&settings, &combinations),
            confirm_set_end: false,
        };
        apply_rules(&mut c, &settings);
        self.current = Some(c);
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
        Ok(c.preview(text))
    }

    /// Adds the codes on the command line (separated by spaces) to the
    /// match, then what follows from them: the point code when a touch
    /// decides the rally, and the winner's rotation on a side-out. All of
    /// it is saved before the new state is returned; if saving fails the
    /// match is left as it was.
    pub fn commit(&mut self, text: &str, clock: Option<ClockTime>) -> Result<Snapshot> {
        let c = self.current.as_mut().ok_or(Error::NoMatch)?;
        let season = self.season.as_mut().ok_or(Error::NoSeason)?;
        if let Some(set) = snapshot::set_end_due(&c.game, &fold(&c.game)) {
            return Err(Error::BadCode(format!(
                "Set {set} is won: end it first (Enter on an empty line), or undo."
            )));
        }
        let preview = c.preview(text);
        if !preview.ok {
            let why = preview
                .codes
                .iter()
                .find_map(|r| r.error.as_ref().map(|e| format!("{}: {e}", r.text)))
                .unwrap_or_else(|| "Type a code first.".into());
            return Err(Error::BadCode(why));
        }
        let before = c.game.log.len();
        // The expanded codes, in order.
        let codes: Vec<Code> = preview.codes.iter().map(|r| Code::parse(&r.text)).collect();
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
            if won(&c.game, &after, set, score) {
                // Held for the scout to confirm (`end_set`) when the season asks.
                if !c.confirm_set_end {
                    c.add(Code::parse(&format!("**{set}set")), Producer::Repair, clock);
                }
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
        let zone = check_six(c.game.teams.get(team), &court, setter)?;
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

    /// Ends the set in progress once its score wins it: the set end the
    /// console holds back when the season asks to confirm set ends.
    pub fn end_set(&mut self, clock: Option<ClockTime>) -> Result<Snapshot> {
        let c = self.current.as_mut().ok_or(Error::NoMatch)?;
        let season = self.season.as_mut().ok_or(Error::NoSeason)?;
        let Some(set) = snapshot::set_end_due(&c.game, &fold(&c.game)) else {
            return Err(Error::BadCode("The set isn't won yet.".into()));
        };
        let before = c.game.log.len();
        c.add(
            Code::parse(&format!("**{set}set")),
            Producer::Keyboard,
            clock,
        );
        c.persist(season, before)?;
        self.snapshot()
    }

    /// Starts a golden set once the match is over: an event of its own,
    /// so undo takes it back. The set goes to 15 and needs lineups.
    pub fn start_golden_set(&mut self, clock: Option<ClockTime>) -> Result<Snapshot> {
        let snap = self.snapshot()?;
        if !snap.match_over || snap.golden_set.is_some() {
            return Err(Error::BadCode(
                "A golden set is played once the match is over, and only one.".into(),
            ));
        }
        let c = self.current.as_mut().ok_or(Error::NoMatch)?;
        let season = self.season.as_mut().ok_or(Error::NoSeason)?;
        let before = c.game.log.len();
        c.game.log.append(
            &mut c.ids,
            NewEvent {
                body: Body::GoldenSet,
                video_ms: None,
                clock,
                producer: Producer::Keyboard,
            },
        );
        c.persist(season, before)?;
        self.snapshot()
    }

    /// Corrects `team`'s starting six for the set in progress (zones 1–6)
    /// and its setter; the rest of the set replays from it. The six entered
    /// for the set is corrected, or for a set whose six came from the team
    /// sheet (an imported file) one is put before the set's first code. The
    /// set's `P` and `z` codes marked `>LUp` follow, written by the console,
    /// so one undo takes back the whole change. With nothing coded in the
    /// set yet, this enters the lineup.
    pub fn correct_lineup(
        &mut self,
        team: Team,
        court: [u8; 6],
        setter: u8,
        clock: Option<ClockTime>,
    ) -> Result<Snapshot> {
        let c = self.current.as_mut().ok_or(Error::NoMatch)?;
        let zone = check_six(c.game.teams.get(team), &court, setter)?;
        let folded = fold(&c.game);
        let visible: Vec<MatchEvent> = c.game.log.visible().into_iter().cloned().collect();
        let set = folded.states.last().map_or(1, |s| s.set);
        let in_set: Vec<&MatchEvent> = visible
            .iter()
            .zip(&folded.states)
            .filter(|(e, s)| s.set == set && e.kind() != EventKind::SetEnd)
            .map(|(e, _)| e)
            .collect();
        let Some(first) = in_set.first() else {
            return self.set_lineup(team, court, setter, clock);
        };
        let season = self.season.as_mut().ok_or(Error::NoSeason)?;
        let before = c.game.log.len();
        let event = |body: Body, producer: Producer| NewEvent {
            body,
            video_ms: None,
            clock,
            producer,
        };
        let six = event(Body::StartingSix { team, court }, Producer::Keyboard);
        let entered = in_set
            .iter()
            .find(|e| matches!(e.body, Body::StartingSix { team: t, .. } if t == team));
        let log = &mut c.game.log;
        match entered {
            Some(e) => log.correct(&mut c.ids, e.id, six)?,
            None => log.insert_before(&mut c.ids, first.id, six)?,
        };
        // The set's lineup codes: the run of them before play starts.
        let lineup = |e: &&&MatchEvent| {
            matches!(
                e.kind(),
                EventKind::Lineup
                    | EventKind::StartingSix
                    | EventKind::Comment
                    | EventKind::GoldenSet
            )
        };
        let t = team.as_char();
        let wanted = [
            Code::parse(&format!("{t}P{setter:02}>LUp")),
            Code::parse(&format!("{t}z{zone}>LUp")),
        ];
        let mut missing = Vec::new();
        for code in wanted {
            let same_kind = |e: &&&MatchEvent| {
                e.kind() == EventKind::Lineup
                    && e.code().is_some_and(|x| {
                        x.team() == Some(team)
                            && std::mem::discriminant(x) == std::mem::discriminant(&code)
                    })
            };
            match in_set.iter().take_while(lineup).find(same_kind) {
                Some(e) if e.code() == Some(&code) => {}
                Some(e) => {
                    log.correct(&mut c.ids, e.id, event(Body::Code(code), Producer::Repair))?;
                }
                None => missing.push(code),
            }
        }
        let play = in_set.iter().find(|e| !lineup(e));
        for code in missing {
            let new = event(Body::Code(code), Producer::Repair);
            match play {
                Some(e) => {
                    log.insert_before(&mut c.ids, e.id, new)?;
                }
                None => {
                    log.append(&mut c.ids, new);
                }
            }
        }
        c.persist(season, before)?;
        self.snapshot()
    }

    /// Replaces an earlier code with `text` (one code), in place.
    ///
    /// When the code is one half of a pair (a serve and the other team's
    /// reception, an attack and the other team's block or dig, one straight
    /// after the other), the other half follows what changed: type,
    /// evaluation through the pairing table, zones. A new attack
    /// combination re-derives the target of the same team's set straight
    /// before it. Those follow-ups are written by the console, so one undo
    /// takes back all of them.
    pub fn correct(&mut self, id: &str, text: &str, clock: Option<ClockTime>) -> Result<Snapshot> {
        let defaults = self
            .current
            .as_ref()
            .map(|c| c.defaults.clone())
            .unwrap_or_default();
        self.edit(id, Some(text), clock, |log, ids, target, new| {
            let Some(new) = new else {
                return Err(LogError::NotFound(target));
            };
            let partner = pair_partner(log, target);
            let set = touch_just_before(log, target);
            let id = log.correct(ids, target, new)?;
            let repair = |t: Touch| NewEvent {
                body: Body::Code(Code::Touch(t)),
                video_ms: None,
                clock,
                producer: Producer::Repair,
            };
            if let Some((set_id, set)) = set
                && let (Some(old), Some(now)) = (
                    log.get(target).and_then(touch_of),
                    log.get(id).and_then(touch_of),
                )
                && let Some(updated) = follow_combination(&old, &now, &set, &defaults)
            {
                log.correct(ids, set_id, repair(updated))?;
            }
            if let Some((partner, edited_first)) = partner {
                let old = log.get(target).and_then(touch_of);
                let now = log.get(id).and_then(touch_of);
                let other = log.get(partner).and_then(touch_of);
                if let (Some(old), Some(now), Some(other)) = (old, now, other)
                    && let Some(updated) = follow_pair(&old, &now, &other, edited_first)
                {
                    log.correct(ids, partner, repair(updated))?;
                }
            }
            Ok(id)
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
                let before = c.touch_before(target);
                let preview =
                    readback::preview(text, &c.game, c.names(), before.as_ref(), &c.defaults);
                if let Some(e) = preview.codes.iter().find_map(|r| r.error.clone()) {
                    return Err(Error::BadCode(format!("{text}: {e}")));
                }
                let [code] = preview.codes.as_slice() else {
                    return Err(Error::BadCode(
                        "A dot code makes two codes; correct one half at a time.".into(),
                    ));
                };
                Some(NewEvent {
                    body: Body::Code(Code::parse(&code.text)),
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
