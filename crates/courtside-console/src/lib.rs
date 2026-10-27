//! The coding console's session (docs/adr/0001): the open season and match,
//! held in the desktop backend. The UI sends codes and commands and gets a
//! [`Snapshot`] back; every change is saved before the snapshot is built.
//!
//! Nothing here knows about Tauri, so the whole loop is tested headless.

pub mod clips;
pub mod keys;
pub mod manage;
pub mod readback;
pub mod resume;
pub mod snapshot;

use std::path::Path;

use courtside_store::Season;
use vb_engine::log::{EventLog, LogError};
use vb_engine::model::Outcome;
use vb_engine::{
    Body, EventId, EventKind, IdGen, Match, MatchEvent, NewEvent, Points, Producer, Score,
    TeamSheet, fold,
};
use vb_grammar::expand::{Combination, Defaults, follow_combination, follow_pair, is_pair};
use vb_grammar::{Code, Skill, Touch};

pub use clips::{ClipFilter, Clips};
pub use courtside_store::{Error as StoreError, MatchId, SKILLS, Settings};
pub use keys::Keys;
pub use manage::{
    PersonView, PlayerDetail, PlayerListItem, StaffView, TeamDetail, TeamForm, TeamImportView,
    VenueView,
};
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

/// When a code was typed: the time of day and, with a video open, the video
/// time.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Stamp {
    pub clock: Option<ClockTime>,
    pub video_ms: Option<u64>,
}

/// What "Fix rotations from here" would change ([`Session::fix_rotations_preview`]).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FixPreview {
    /// Rotation codes written, corrected or removed.
    pub codes: usize,
    /// The first and last rally they're in.
    pub from_rally: Option<u16>,
    pub to_rally: Option<u16>,
    /// Issues in the match now and after the fix.
    pub issues_before: usize,
    pub issues_after: usize,
}

/// One change to the console's side-out rotations ([`Current::rerotate`]).
enum RotationFix {
    Correct(EventId, Code),
    Remove(EventId),
    /// Goes in before the event (at the end of the log when `None`).
    Insert(Option<EventId>, Code, Stamp),
}

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
    /// Log lengths where a change made only of console codes begins (a
    /// fix the scout asked for): undo stops there, not at the code before.
    marks: Vec<usize>,
    /// Typing defaults (season settings, step E).
    defaults: Defaults,
    /// The match's own attack combinations (empty: the season's are used).
    own_combinations: Vec<Combination>,
    /// Hold set ends until the scout confirms (season settings).
    confirm_set_end: bool,
    /// How codes get their video time (season settings).
    video: courtside_store::Video,
    /// The match video's path (saved with the match).
    video_path: Option<String>,
    /// Whether the match was over at the last snapshot. Every command ends
    /// with one, so a commit reads this instead of building another.
    match_over: std::cell::Cell<Option<bool>>,
}

/// Each event's video time once the match's first serve is at
/// `serve_ms` (see [`Session::sync_video`]).
fn sync_plan(log: &EventLog, serve_ms: u64) -> Result<Vec<(EventId, Option<u64>)>> {
    let visible = log.visible();
    let serve = visible
        .iter()
        .find(|e| matches!(e.code(), Some(Code::Touch(t)) if t.skill == Skill::Serve))
        .ok_or_else(|| Error::BadCode("There's no serve to sync the video to yet.".into()))?;
    const DAY: i64 = 24 * 3600 * 1000;
    let day_ms = |c: ClockTime| {
        (i64::from(c.hour) * 3600 + i64::from(c.minute) * 60 + i64::from(c.second)) * 1000
    };
    let target = i64::try_from(serve_ms).unwrap_or(i64::MAX);
    let by_video = serve
        .video_ms
        .map(|v| target - i64::try_from(v).unwrap_or(0));
    let by_clock = serve.clock.map(|c| target - day_ms(c));
    if by_video.is_none() && by_clock.is_none() {
        return Err(Error::BadCode(
            "The first serve has no time to sync from.".into(),
        ));
    }
    let serve_day = serve.clock.map(day_ms);
    let to_ms = |v: i64| Some(u64::try_from(v.max(0)).unwrap_or(0));
    Ok(log
        .all()
        .iter()
        .filter_map(|e| {
            if let (Some(shift), Some(v)) = (by_video, e.video_ms) {
                return Some((e.id, to_ms(i64::try_from(v).unwrap_or(0) + shift)));
            }
            let (offset, c) = (by_clock?, e.clock?);
            // Past midnight from the serve, or before it.
            let mut at = day_ms(c);
            if let Some(s) = serve_day {
                if at - s < -DAY / 2 {
                    at += DAY;
                } else if at - s > DAY / 2 {
                    at -= DAY;
                }
            }
            Some((e.id, to_ms(at + offset)))
        })
        .collect())
}

/// Predictive mode over the whole match: each rally's touches after its
/// first get the first's video time plus `step` per touch. Only touches
/// whose time changes.
fn predicted(log: &EventLog, step: u64) -> Vec<(EventId, u64)> {
    let (mut first, mut touches) = (None, 0);
    let mut out = Vec::new();
    for e in log.visible() {
        match e.kind() {
            EventKind::Point | EventKind::SetEnd => (first, touches) = (None, 0),
            EventKind::Touch => {
                if touches == 0 {
                    first = e.video_ms;
                } else if let Some(t0) = first {
                    let want = t0 + touches * step;
                    if e.video_ms != Some(want) {
                        out.push((e.id, want));
                    }
                }
                touches += 1;
            }
            _ => {}
        }
    }
    out
}

/// Whether `score` wins `set` under the match's regulation (a golden set
/// goes to 15).
pub(crate) fn won(
    game: &Match,
    folded: &vb_engine::Folded,
    set: u8,
    score: vb_engine::Score,
) -> bool {
    let target = folded.set_target(game, set);
    let (hi, lo) = (
        score.home.max(score.visiting),
        score.home.min(score.visiting),
    );
    hi >= target && hi - lo >= 2
}

/// The command line's defaults: typing from the season settings, attack
/// combinations from the match's own table (`own`), or the season's when
/// the match has none (DataVolley files often leave the table empty).
fn defaults_from(s: &Settings, own: &[Combination]) -> Defaults {
    let skill = |c: char| Skill::from_char(c).unwrap_or(Skill::Attack);
    let combinations = if own.is_empty() {
        s.tables
            .combinations
            .iter()
            .map(|c| Combination {
                code: c.code.clone(),
                tempo: c.tempo,
                start_zone: char::from_digit(u32::from(c.start_zone), 10).unwrap_or('~'),
                target: c.target.chars().next(),
            })
            .collect()
    } else {
        own.to_vec()
    };
    Defaults {
        skill: skill(s.typing.skill),
        types: s.typing.types,
        evaluations: s.typing.evaluations,
        predict: s.typing.predict,
        combinations,
    }
}

/// A match's attack combinations as expansion takes them: the imported file's `[3ATTACKCOMBINATION]`, or for a match
/// coded here the table frozen into its blank template when it was created.
/// Like DataVolley, a match uses its own table when it has one.
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

/// The team the last deciding touch among `codes` gives the rally to.
fn decided_by<'a>(game: &Match, codes: impl DoubleEndedIterator<Item = &'a Code>) -> Option<Team> {
    codes.rev().find_map(|code| match code {
        Code::Touch(t) => {
            let o = game.winning_symbols.outcome(t.skill, t.evaluation()?)?;
            Some(if o == Outcome::Win {
                t.team
            } else {
                other(t.team)
            })
        }
        _ => None,
    })
}

/// The green code (`$$&`) DataVolley and VolleyStation write for a rally
/// that `winner` won and whose last touch is `last`, when that touch has no
/// counterpart from the other team: the loser's `$$&H=` after the winner's
/// ace or kill, the winner's `$$&H#` after the loser's serve, attack, set or
/// freeball error. A dig, reception or block error already pairs with the
/// winner's touch before it. The rule comes from the corpus (FORMAT-NOTES).
fn green_for(last: &Touch, winner: Team) -> Option<Code> {
    let text = match (last.team == winner, last.skill, last.evaluation()?) {
        (true, Skill::Serve | Skill::Attack, '#') => format!("{}$$&H=", other(winner).as_char()),
        (false, Skill::Serve | Skill::Attack | Skill::Set | Skill::Freeball, '=') => {
            format!("{}$$&H#", winner.as_char())
        }
        _ => return None,
    };
    Some(Code::parse(&text))
}

fn is_green(e: &MatchEvent) -> bool {
    matches!(e.code(), Some(Code::Touch(t)) if t.skill == Skill::Green)
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

/// The season's rule options onto the open match.
fn apply_rules(c: &mut Current, s: &Settings) {
    c.game.rules = vb_engine::Rules {
        substitutions: s.rules.substitutions,
        libero_may_serve: s.rules.libero_may_serve,
    };
    c.confirm_set_end = s.rules.confirm_set_end;
    c.video = s.video;
}

/// Settings the scout can't save: what a `.dvw` couldn't hold.
fn check_settings(s: &Settings) -> Result<()> {
    let bad = |m: String| Err(Error::BadCode(m));
    if !(200..=5000).contains(&s.video.step_ms) {
        return bad("The step between touches is 0.2 to 5 seconds.".into());
    }
    if s.video.pre_roll_ms > 30_000 || s.video.post_roll_ms > 30_000 {
        return bad("A clip's time before and after its code is 0 to 30 seconds.".into());
    }
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
/// set end. Green codes aren't touches.
fn last_touch<'a>(events: impl Iterator<Item = &'a &'a MatchEvent>) -> Option<Touch> {
    for e in events {
        match e.code() {
            Some(Code::Touch(t)) if t.skill != Skill::Green => return Some(t.clone()),
            Some(Code::Point { .. } | Code::SetEnd { .. }) => return None,
            _ => {}
        }
    }
    None
}

/// Where a play started when only its end is marked: the middle of its
/// start zone in the play's frame (`vb_engine::court`), from where the
/// corpus's marked plays start (FORMAT-NOTES). A serve starts behind the
/// baseline.
fn zone_start(t: &Touch) -> Option<(f64, f64)> {
    let z = t.start_zone()?.to_digit(10)?;
    if t.skill == Skill::Serve {
        let x = match z {
            5 => 0.95,
            7 => 1.25,
            6 => 2.0,
            9 => 2.7,
            1 => 3.05,
            _ => return None,
        };
        return Some((x, 0.2));
    }
    let x = match z {
        4 | 7 | 5 => 1.0,
        3 | 8 | 6 => 2.0,
        2 | 9 | 1 => 3.0,
        _ => return None,
    };
    let y = match z {
        2..=4 => 3.1,
        7..=9 => 2.5,
        _ => 1.2,
    };
    Some((x, y))
}

/// Notes an attack whose start zone isn't its combination's (a court mark
/// or a typed zone over a combination that starts elsewhere): one of the
/// two is likely a slip.
fn combination_zones(snap: &mut Snapshot, game: &Match, table: &[Combination]) {
    for (index, e) in game.log.visible().iter().enumerate() {
        let Some(t) = touch_of(e).filter(|t| t.skill == Skill::Attack) else {
            continue;
        };
        let (Some(code), Some(zone)) = (t.combination(), t.start_zone()) else {
            continue;
        };
        let Some(c) = table
            .iter()
            .find(|c| c.code == code && c.start_zone != '~' && c.start_zone != zone)
        else {
            continue;
        };
        let id = e.id.0.to_string();
        snap.issues.push(snapshot::IssueView {
            severity: "info",
            rule: "code",
            message: format!(
                "{t}: {code} starts in zone {}, the code in zone {zone}; check the combination",
                c.start_zone
            ),
            event_id: id.clone(),
            index,
            knock_on: false,
            cause_id: None,
        });
        if let Some(r) = snap
            .groups
            .iter_mut()
            .flat_map(|g| &mut g.rows)
            .find(|r| r.id == id)
        {
            r.severity.get_or_insert("info");
        }
    }
    snap.issues.sort_by_key(|i| i.index);
}

/// `t` with the zones its court mark gives it (GitHub #11), over what was
/// typed: a serve's (and its reception's) serve zone from where it
/// started and the end zone and subzone where it landed; an attack's,
/// dig's or free ball's start and end zones and end subzone; a set's or
/// block's zone where it was, in the end-zone place as `.dvw` keeps it.
/// The start zone only comes from a mark that has its own start
/// (`started`). A play that ended off the court has no end zone.
fn mark_zones(t: &Touch, points: Points, started: bool) -> Touch {
    use vb_engine::court;
    let digit = |z: u8| char::from(b'0' + z);
    let at = |cell: Option<u16>| cell.and_then(court::position);
    let (from, to) = (at(points.start).filter(|_| started), at(points.end));
    let landed = |t: Touch| match to {
        Some((x, y)) if court::on_court(x, y) => t
            .with_end_zone(digit(court::zone(x, y)))
            .with_end_subzone(court::subzone(x, y)),
        Some(_) => t.with_end_zone('~').with_end_subzone('~'),
        None => t,
    };
    let t = t.clone();
    match t.skill {
        Skill::Serve | Skill::Reception => landed(match from {
            Some((x, y)) => t.with_start_zone(digit(court::serve_zone(x, y))),
            None => t,
        }),
        Skill::Attack | Skill::Dig | Skill::Freeball => landed(match from {
            Some((x, y)) => t.with_start_zone(digit(court::zone(x, y))),
            None => t,
        }),
        Skill::Set | Skill::Block => match from {
            Some((x, y)) => t.with_end_zone(digit(court::zone(x, y))),
            None => t,
        },
        Skill::Green => t,
    }
}

/// `t` (event `id`) with the zones it had before it was marked: those of
/// the last version of it a mark didn't write.
fn unmarked_zones(log: &EventLog, id: EventId, t: &Touch) -> Touch {
    let mut cur = log.get(id);
    while let Some(e) = cur
        && matches!(e.producer, Producer::Tap | Producer::Repair)
        && let Some(prev) = e.supersedes
    {
        cur = log.get(prev);
    }
    let Some(typed) = cur.and_then(touch_of) else {
        return t.clone();
    };
    let z = |c: Option<char>| c.unwrap_or('~');
    t.clone()
        .with_start_zone(z(typed.start_zone()))
        .with_end_zone(z(typed.end_zone()))
        .with_end_subzone(z(typed.end_subzone()))
}

/// Who serves the rally that starts after the first `upto` visible events:
/// the last rally's winner in the set, and its player in zone 1. Unknown
/// before a set's first point or without the six.
fn server_at(folded: &vb_engine::Folded, upto: usize) -> Option<(Team, u8)> {
    let st = folded.states.get(upto.checked_sub(1)?)?;
    let team = folded
        .rallies
        .iter()
        .rev()
        .filter(|r| r.last < upto && r.set == st.set)
        .find_map(|r| r.won_by)?;
    let server = st.court.get(team)[0]?;
    Some((team, server))
}

/// The team that served rally `r`: the last rally's winner in the set,
/// as the rotation has it (a serve typed for the other team is the slip,
/// and the checks say so); the set's first rally's serve, or `None`.
fn server_of(folded: &vb_engine::Folded, r: usize) -> Option<Team> {
    let now = folded.rallies.get(r)?;
    folded.rallies[..r]
        .iter()
        .rev()
        .take_while(|x| x.set == now.set)
        .find_map(|x| x.won_by)
        .or(now.serving)
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
        let folded = fold(&self.game);
        let setters = folded.states.last().map(|s| s.setter);
        readback::preview(
            text,
            &self.game,
            self.names(),
            before.as_ref(),
            &self.defaults,
            Some(setters.unwrap_or_default()),
            server_at(&folded, folded.states.len()),
        )
    }

    /// The open rally's first touch's video time and how many touches it
    /// has so far.
    fn rally_so_far(&self) -> (Option<u64>, u64) {
        let (mut first, mut touches) = (None, 0);
        for e in self.game.log.visible().iter().rev() {
            match e.kind() {
                EventKind::Point | EventKind::SetEnd => break,
                EventKind::Touch => {
                    first = e.video_ms;
                    touches += 1;
                }
                _ => {}
            }
        }
        (first, touches)
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
        let root = self.position_of(target)?;
        last_touch(visible[..root].iter().rev())
    }

    /// Where `target` (or the correction standing in its place) is among
    /// the visible events.
    fn position_of(&self, target: EventId) -> Option<usize> {
        self.game.log.visible().iter().position(|e| {
            e.id == target || self.game.log.get(e.id).and_then(|x| x.supersedes) == Some(target)
        })
    }

    /// The command line read back as codes going in just before `target`:
    /// the touches and setters there are the context. A line of more than
    /// one code gets the setter's set like the command line; one code in
    /// is one code out.
    fn preview_before(&self, text: &str, target: EventId) -> Option<Preview> {
        let pos = self.position_of(target)?;
        let setters = (text.split_whitespace().count() > 1).then(|| {
            pos.checked_sub(1)
                .and_then(|p| fold(&self.game).states.get(p).map(|s| s.setter))
                .unwrap_or_default()
        });
        let before = self.touch_before(target);
        Some(readback::preview(
            text,
            &self.game,
            self.names(),
            before.as_ref(),
            &self.defaults,
            setters,
            server_at(&fold(&self.game), pos),
        ))
    }

    /// Point codes carry the score. After a rally goes in mid-set, each
    /// later point code of the set is rewritten (by the console) to the
    /// score it now stands at, counting on from `score` at visible event
    /// `from`.
    fn rescore_from(&mut self, from: usize, mut score: Score) -> std::result::Result<(), LogError> {
        let points: Vec<(EventId, Option<Code>)> = self.game.log.visible()[from..]
            .iter()
            .map(|e| (e.id, e.code().cloned()))
            .collect();
        for (id, code) in points {
            match code {
                Some(Code::Point { team, .. }) => {
                    match team {
                        Team::Home => score.home = score.home.saturating_add(1),
                        Team::Visiting => score.visiting = score.visiting.saturating_add(1),
                    }
                    let now = format!("{}p{:02}:{:02}", team.as_char(), score.home, score.visiting);
                    if code.is_some_and(|c| c.to_string() != now) {
                        self.game.log.correct(
                            &mut self.ids,
                            id,
                            NewEvent::code(Code::parse(&now), Producer::Repair),
                        )?;
                    }
                }
                Some(Code::SetEnd { .. }) => break,
                _ => {}
            }
        }
        Ok(())
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

    fn add(&mut self, code: Code, producer: Producer, at: Stamp) {
        let event = NewEvent {
            body: Body::Code(code),
            video_ms: at.video_ms,
            clock: at.clock,
            producer,
        };
        self.game.log.append(&mut self.ids, event);
    }

    /// What the console writes when a rally is over: the point (unless the
    /// scout typed it), then the set end when the set is won (held when the
    /// season asks to confirm it, unless `keep_set_end`: the scout already
    /// confirmed it) or the winner's rotation after a side-out.
    fn close_rally(
        &mut self,
        typed_point: Option<Team>,
        decided: Option<Team>,
        keep_set_end: bool,
        at: Stamp,
    ) {
        let Some(winner) = typed_point.or(decided) else {
            return;
        };
        let folded = fold(&self.game);
        let serving = folded
            .rallies
            .len()
            .checked_sub(1)
            .and_then(|r| server_of(&folded, r));
        let last = folded.states.last();
        if typed_point.is_none() {
            let score = last.map(|s| s.score).unwrap_or_default();
            let (h, v) = match winner {
                Team::Home => (score.home.saturating_add(1), score.visiting),
                Team::Visiting => (score.home, score.visiting.saturating_add(1)),
            };
            let point = format!("{}p{h:02}:{v:02}", winner.as_char());
            self.add(Code::parse(&point), Producer::Repair, at);
        }
        let after = fold(&self.game);
        let end = after.states.last();
        let set = end.map_or(1, |s| s.set);
        let score = end.map(|s| s.score).unwrap_or_default();
        if won(&self.game, &after, set, score) {
            // Held for the scout to confirm (`end_set`) when the season asks.
            if !self.confirm_set_end || keep_set_end {
                self.add(Code::parse(&format!("**{set}set")), Producer::Repair, at);
            }
        } else {
            // Side-out: the winner rotates, its setter one zone on.
            let zone = last.and_then(|s| *s.setter_position.get(winner));
            if let (true, Some(z)) = (serving.is_some_and(|s| s != winner), zone) {
                let next = if z == 1 { 6 } else { z - 1 };
                let rotation = format!("{}z{next}", winner.as_char());
                self.add(Code::parse(&rotation), Producer::Repair, at);
            }
        }
    }

    /// The console's side-out rotations for `team` in `set`, written again
    /// from the court as it now stands (after its lineup was corrected):
    /// each one moves the setter one zone on from where it was.
    fn rotations_follow(&mut self, team: Team, set: u8) -> std::result::Result<(), LogError> {
        for _ in 0..self.game.log.len() {
            let wrong = {
                let folded = fold(&self.game);
                let visible = self.game.log.visible();
                visible.iter().enumerate().find_map(|(i, e)| {
                    if e.producer != Producer::Repair
                        || e.kind() != EventKind::SetterPosition
                        || folded.states.get(i)?.set != set
                        || e.code().and_then(Code::team) != Some(team)
                    {
                        return None;
                    }
                    let z = (*folded
                        .states
                        .get(i.checked_sub(1)?)?
                        .setter_position
                        .get(team))?;
                    let next = if z == 1 { 6 } else { z - 1 };
                    let want = Code::parse(&format!("{}z{next}", team.as_char()));
                    (e.code() != Some(&want)).then_some((e.id, want))
                })
            };
            let Some((id, want)) = wrong else {
                return Ok(());
            };
            // The correction keeps the code's times.
            self.game
                .log
                .correct(&mut self.ids, id, NewEvent::code(want, Producer::Repair))?;
        }
        Ok(())
    }

    /// The side-out rotations the console wrote, written again from the
    /// rally winners for the set at visible event `from`, from the rally
    /// before it to the set's end: after each point the winner of a
    /// side-out rotates once, and nobody rotates otherwise. A rotation the
    /// scout typed (or a console one they edited) stands in place of the
    /// console's and is never changed; the checks say when it disagrees.
    fn rerotate(&mut self, from: usize) -> std::result::Result<(), LogError> {
        for _ in 0..=self.game.log.len() {
            let Some(fix) = self.rotation_fix(from) else {
                return Ok(());
            };
            match fix {
                RotationFix::Correct(id, code) => {
                    self.game.log.correct(
                        &mut self.ids,
                        id,
                        NewEvent::code(code, Producer::Repair),
                    )?;
                }
                RotationFix::Remove(id) => {
                    self.game.log.remove(&mut self.ids, id, Producer::Repair)?;
                }
                RotationFix::Insert(before, code, at) => {
                    let new = NewEvent {
                        body: Body::Code(code),
                        video_ms: at.video_ms,
                        clock: at.clock,
                        producer: Producer::Repair,
                    };
                    match before {
                        Some(id) => {
                            self.game.log.insert_before(&mut self.ids, id, new)?;
                        }
                        None => {
                            self.game.log.append(&mut self.ids, new);
                        }
                    }
                }
            }
        }
        Ok(())
    }

    /// The first console rotation [`Current::rerotate`] would change.
    fn rotation_fix(&self, from: usize) -> Option<RotationFix> {
        let folded = fold(&self.game);
        let visible = self.game.log.visible();
        let set = folded
            .states
            .get(from.min(visible.len().checked_sub(1)?))?
            .set;
        let in_set = |i: usize| folded.states.get(i).is_some_and(|s| s.set == set);
        // From the point before `from`: an edit there decides whether the
        // rotation after that point is still due.
        let mut start = from.min(visible.len());
        while start > 0 && in_set(start - 1) && visible[start - 1].kind() != EventKind::Point {
            start -= 1;
        }
        start = start.saturating_sub(1);
        for i in start..visible.len() {
            if !in_set(i) {
                if i > from {
                    break;
                }
                continue;
            }
            let Some(Code::Point { team: winner, .. }) = visible[i].code() else {
                continue;
            };
            let winner = *winner;
            // The rotation codes between this point and the next play.
            let mut gap = Vec::new();
            let mut j = i + 1;
            while j < visible.len()
                && in_set(j)
                && !matches!(
                    visible[j].kind(),
                    EventKind::Touch | EventKind::Point | EventKind::SetEnd
                )
            {
                if visible[j].kind() == EventKind::SetterPosition
                    && matches!(visible[j].code(), Some(Code::SetterPosition { note, .. }) if note.is_empty())
                {
                    gap.push(visible[j]);
                }
                j += 1;
            }
            let st = &folded.states[i];
            let over = visible
                .get(j)
                .is_some_and(|e| e.kind() == EventKind::SetEnd)
                || won(&self.game, &folded, set, st.score);
            let serving = st.rally.and_then(|r| server_of(&folded, r));
            let side_out = !over && serving.is_some_and(|s| s != winner);
            let want = if side_out {
                // Where the winner's setter is unknown nothing can be said.
                let Some(z) = *st.setter_position.get(winner) else {
                    continue;
                };
                let next = if z == 1 { 6 } else { z - 1 };
                Some(Code::parse(&format!("{}z{next}", winner.as_char())))
            } else {
                None
            };
            let theirs = |e: &&MatchEvent| e.producer != Producer::Repair;
            let team_of = |e: &&MatchEvent| e.code().and_then(Code::team);
            let own = gap.iter().any(|e| theirs(e) && team_of(e) == Some(winner));
            let mut console = gap.iter().filter(|e| !theirs(e));
            match want {
                Some(want) if !own => {
                    let keep = gap
                        .iter()
                        .find(|e| !theirs(e) && team_of(e) == Some(winner));
                    match keep {
                        Some(e) if e.code() != Some(&want) => {
                            return Some(RotationFix::Correct(e.id, want));
                        }
                        Some(e) => {
                            if let Some(extra) = console.find(|x| x.id != e.id) {
                                return Some(RotationFix::Remove(extra.id));
                            }
                        }
                        None => {
                            if let Some(extra) = console.next() {
                                return Some(RotationFix::Remove(extra.id));
                            }
                            let at = Stamp {
                                clock: visible[i].clock,
                                video_ms: visible[i].video_ms,
                            };
                            let before = visible.get(i + 1).map(|e| e.id);
                            return Some(RotationFix::Insert(before, want, at));
                        }
                    }
                }
                _ => {
                    if let Some(extra) = console.next() {
                        return Some(RotationFix::Remove(extra.id));
                    }
                }
            }
        }
        None
    }

    /// After an edit, writes again what the console wrote at the end of the
    /// last rally (point, rotation, set end), from the rally as it now
    /// stands: a deciding code modified to the other team moves the point,
    /// a set no longer won loses its set end, a rally now decided gets
    /// closed. Only the last rally: a point earlier in the match carries a
    /// score every later point code repeats. Nothing changes when the
    /// result is the same. The codes written again keep the rally's times.
    fn reclose(&mut self, clock: Option<ClockTime>) -> std::result::Result<(), LogError> {
        let visible: Vec<MatchEvent> = self.game.log.visible().into_iter().cloned().collect();
        let folded = fold(&self.game);
        // The console's codes, and a set end the scout confirmed (`end_set`)
        // on a set the score had won; a set end typed on any other score
        // is the scout's own and stays.
        let console_end = |i: usize| {
            let e = &visible[i];
            match e.kind() {
                EventKind::Point | EventKind::SetterPosition => e.producer == Producer::Repair,
                EventKind::SetEnd => {
                    e.producer == Producer::Repair
                        || i.checked_sub(1).is_some_and(|p| {
                            let st = &folded.states[p];
                            won(&self.game, &folded, st.set, st.score)
                        })
                }
                _ => false,
            }
        };
        let mut k = visible.len();
        while k > 0 && console_end(k - 1) {
            k -= 1;
        }
        let typed_point = k.checked_sub(1).and_then(|i| match visible[i].code() {
            Some(Code::Point { team, .. }) => Some(*team),
            _ => None,
        });
        let rally_end = if typed_point.is_some() { k - 1 } else { k };
        let mut j = rally_end;
        while j > 0 && !matches!(visible[j - 1].kind(), EventKind::Point | EventKind::SetEnd) {
            j -= 1;
        }
        // The console's own green code follows the ending, not the other
        // way round (`regreen`).
        let decided = decided_by(
            &self.game,
            visible[j..rally_end]
                .iter()
                .filter(|e| !(is_green(e) && e.producer == Producer::Repair))
                .filter_map(MatchEvent::code),
        );
        let old: Vec<String> = visible[k..]
            .iter()
            .filter_map(|e| e.code().map(Code::to_string))
            .collect();
        if old.is_empty() && typed_point.or(decided).is_none() {
            return Ok(());
        }
        let keep_set_end = visible[k..].iter().any(|e| e.kind() == EventKind::SetEnd);
        let rally = &visible[j..k];
        let at = Stamp {
            clock: rally.iter().rev().find_map(|e| e.clock).or(clock),
            video_ms: rally.iter().rev().find_map(|e| e.video_ms),
        };
        let before = self.game.log.len();
        for e in &visible[k..] {
            self.game
                .log
                .remove(&mut self.ids, e.id, Producer::Repair)?;
        }
        self.close_rally(typed_point, decided, keep_set_end, at);
        // The codes after the rally's own, as they now stand.
        let now: Vec<String> = {
            let v = self.game.log.visible();
            v[k.min(v.len())..]
                .iter()
                .filter_map(|e| e.code().map(Code::to_string))
                .collect()
        };
        if now == old {
            while self.game.log.len() > before {
                self.game.log.undo();
            }
        }
        Ok(())
    }

    /// Writes the console's green code again for the rally ending at the
    /// first point code from visible event `from` on (`green_for`), just
    /// before that point code, with its last touch's times. A rally with a
    /// green code the scout typed or imported is left alone.
    fn regreen(&mut self, from: usize) -> std::result::Result<(), LogError> {
        let visible: Vec<MatchEvent> = self.game.log.visible().into_iter().cloned().collect();
        let Some(p) = (from..visible.len()).find(|&i| visible[i].kind() == EventKind::Point) else {
            return Ok(());
        };
        let Some(Code::Point { team: winner, .. }) = visible[p].code() else {
            return Ok(());
        };
        let mut j = p;
        while j > 0 && !matches!(visible[j - 1].kind(), EventKind::Point | EventKind::SetEnd) {
            j -= 1;
        }
        let rally = &visible[j..p];
        if rally
            .iter()
            .any(|e| is_green(e) && e.producer != Producer::Repair)
        {
            return Ok(());
        }
        let last = rally.iter().rev().find_map(|e| {
            touch_of(e)
                .filter(|t| t.skill != Skill::Green)
                .map(|t| (e, t))
        });
        let want = last.and_then(|(e, t)| Some((e, green_for(&t, *winner)?)));
        let have: Vec<&MatchEvent> = rally.iter().filter(|e| is_green(e)).collect();
        if let ([only], Some((_, code))) = (have.as_slice(), &want)
            && only.code() == Some(code)
            && only.id == visible[p - 1].id
        {
            return Ok(());
        }
        if have.is_empty() && want.is_none() {
            return Ok(());
        }
        for e in have {
            self.game
                .log
                .remove(&mut self.ids, e.id, Producer::Repair)?;
        }
        if let Some((touch, code)) = want {
            let event = NewEvent {
                body: Body::Code(code),
                video_ms: touch.video_ms,
                clock: touch.clock,
                producer: Producer::Repair,
            };
            self.game
                .log
                .insert_before(&mut self.ids, visible[p].id, event)?;
        }
        Ok(())
    }

    /// Takes out a console green code from a rally still open: an edit
    /// undid its ending.
    fn drop_open_green(&mut self) -> std::result::Result<(), LogError> {
        let visible: Vec<MatchEvent> = self.game.log.visible().into_iter().cloned().collect();
        let open = visible
            .iter()
            .rposition(|e| matches!(e.kind(), EventKind::Point | EventKind::SetEnd))
            .map_or(0, |p| p + 1);
        for e in &visible[open..] {
            if is_green(e) && e.producer == Producer::Repair {
                self.game
                    .log
                    .remove(&mut self.ids, e.id, Producer::Repair)?;
            }
        }
        Ok(())
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
            c.defaults = defaults_from(settings, &c.own_combinations);
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
    /// coded here, and opens it. `date` is ISO `YYYY-MM-DD`; `venue`
    /// `None` takes the home team's.
    pub fn new_match(
        &mut self,
        home_team_id: &str,
        visiting_team_id: &str,
        date: Option<&str>,
        venue: Option<&VenueView>,
        now_ms: u64,
    ) -> Result<Snapshot> {
        let venue = venue.map(courtside_store::Venue::from);
        let id = self.season_mut()?.create_match(
            home_team_id,
            visiting_team_id,
            date,
            venue.as_ref(),
            now_ms,
        )?;
        self.open_match(id, now_ms)
    }

    /// Where the open match is played.
    pub fn match_venue(&self) -> Result<VenueView> {
        let c = self.current.as_ref().ok_or(Error::NoMatch)?;
        Ok(self.season()?.match_venue(c.id)?.into())
    }

    /// Sets where the open match is played.
    pub fn set_match_venue(&mut self, venue: &VenueView) -> Result<()> {
        let id = self.current.as_ref().ok_or(Error::NoMatch)?.id;
        Ok(self.season_mut()?.set_match_venue(id, &venue.into())?)
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
            marks: Vec::new(),
            defaults: defaults_from(&settings, &combinations),
            own_combinations: combinations.clone(),
            confirm_set_end: false,
            video: courtside_store::Video::default(),
            video_path: season.match_video(id)?,
            match_over: std::cell::Cell::new(None),
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
        let mut snap = snapshot::build(&c.id.to_string(), &c.game, &c.teams);
        snap.video_path.clone_from(&c.video_path);
        combination_zones(&mut snap, &c.game, &c.defaults.combinations);
        c.match_over.set(Some(snap.match_over));
        Ok(snap)
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
        self.commit_at(text, clock, &[])
    }

    /// [`Session::commit`] with the video time at the first keystroke of
    /// each code on the line (space-separated, in order; empty with no
    /// video). In predictive mode (season settings) a touch after the
    /// rally's first gets the first touch's time plus a fixed step per
    /// touch instead. What the console writes after the codes gets the last
    /// one's times.
    pub fn commit_at(
        &mut self,
        text: &str,
        clock: Option<ClockTime>,
        video: &[Option<u64>],
    ) -> Result<Snapshot> {
        let over = self.current.as_ref().and_then(|c| c.match_over.get());
        if over.map_or_else(|| self.snapshot().map(|s| s.match_over), Ok)? {
            return Err(Error::BadCode(
                "The match is over: codes can be modified, or start a golden set.".into(),
            ));
        }
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
        let first_new = c.game.log.visible().len();
        // The expanded codes, in order, with the token each came from.
        let codes: Vec<(Code, usize)> = preview
            .codes
            .iter()
            .map(|r| (Code::parse(&r.text), r.token))
            .collect();
        let typed_point = codes.iter().rev().find_map(|(code, _)| match code {
            Code::Point { team, .. } => Some(*team),
            _ => None,
        });
        let decided = decided_by(&c.game, codes.iter().map(|(code, _)| code));
        let step = c.video.predictive.then_some(u64::from(c.video.step_ms));
        let (mut first, mut touches) = c.rally_so_far();
        let mut last = Stamp {
            clock,
            video_ms: None,
        };
        for (code, token) in codes {
            let typed = video.get(token).copied().flatten();
            let video_ms = match (&code, step, first) {
                (Code::Touch(_), Some(step), Some(t0)) if touches > 0 => Some(t0 + touches * step),
                _ => typed,
            };
            match code {
                Code::Touch(_) => {
                    if touches == 0 {
                        first = video_ms;
                    }
                    touches += 1;
                }
                Code::Point { .. } => (first, touches) = (None, 0),
                _ => {}
            }
            last = Stamp { clock, video_ms };
            c.add(code, Producer::Keyboard, last);
        }
        c.close_rally(typed_point, decided, false, last);
        if let Err(e) = c.regreen(first_new) {
            while c.game.log.len() > before {
                c.game.log.undo();
            }
            return Err(e.into());
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
            Stamp {
                clock,
                video_ms: None,
            },
        );
        c.add(
            Code::parse(&format!("{t}z{zone}>LUp")),
            Producer::Repair,
            Stamp {
                clock,
                video_ms: None,
            },
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
        while !(!taken.is_empty() && c.marks.contains(&c.game.log.len()))
            && let Some(e) = c.game.log.undo()
        {
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
        let len = c.game.log.len();
        c.marks.retain(|m| *m < len);
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
        if events.iter().all(|e| e.producer == Producer::Repair) {
            c.marks.push(before);
        }
        self.snapshot()
    }

    /// Syncs the match to its video (PRD F-20): the match's first serve is
    /// at `serve_ms` in the video. Codes that have a video time move with
    /// the serve; codes without one (coded live, before there was a video)
    /// get it from their time of day, counted from the serve's. In
    /// predictive mode each rally's later touches then follow its first
    /// touch by the season's step. Video times are stamps, not codes, so
    /// this isn't undone: sync again to move them.
    pub fn sync_video(&mut self, serve_ms: u64) -> Result<Snapshot> {
        let c = self.current.as_mut().ok_or(Error::NoMatch)?;
        let season = self.season.as_mut().ok_or(Error::NoSeason)?;
        let plan = sync_plan(&c.game.log, serve_ms)?;
        for (id, ms) in plan {
            c.game.log.set_video_ms(id, ms)?;
        }
        if c.video.predictive {
            for (id, ms) in predicted(&c.game.log, u64::from(c.video.step_ms)) {
                c.game.log.set_video_ms(id, Some(ms))?;
            }
        }
        season.save_video_times(c.id, &c.game.log)?;
        self.snapshot()
    }

    /// Ends the set in progress once its score wins it: the set end the
    /// console holds back when the season asks to confirm set ends.
    /// `minutes`: the set's length as the scout gave it, which the export
    /// writes in place of the clock's (#21).
    pub fn end_set(&mut self, clock: Option<ClockTime>, minutes: Option<u16>) -> Result<Snapshot> {
        let c = self.current.as_mut().ok_or(Error::NoMatch)?;
        let season = self.season.as_mut().ok_or(Error::NoSeason)?;
        let Some(set) = snapshot::set_end_due(&c.game, &fold(&c.game)) else {
            return Err(Error::BadCode("The set isn't won yet.".into()));
        };
        let before = c.game.log.len();
        c.add(
            Code::parse(&format!("**{set}set")),
            Producer::Keyboard,
            Stamp {
                clock,
                video_ms: None,
            },
        );
        c.persist(season, before)?;
        if let Some(end) = c.game.log.all().last().map(|e| e.id)
            && minutes.is_some()
        {
            c.game.log.set_minutes(end, minutes)?;
            season.save_minutes(c.id, end, minutes)?;
        }
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

    /// Corrects `team`'s starting six for `set` (zones 1–6) and its
    /// setter; the rest of the match replays from it. The six entered
    /// for the set is corrected, or for a set whose six came from the team
    /// sheet (an imported file) one is put before the set's first code. The
    /// set's `P` and `z` codes marked `>LUp` follow, written by the console,
    /// so one undo takes back the whole change. With nothing coded in the
    /// set in progress yet, this enters its lineup.
    pub fn correct_lineup(
        &mut self,
        team: Team,
        set: u8,
        court: [u8; 6],
        setter: u8,
        clock: Option<ClockTime>,
    ) -> Result<Snapshot> {
        let c = self.current.as_mut().ok_or(Error::NoMatch)?;
        let zone = check_six(c.game.teams.get(team), &court, setter)?;
        let folded = fold(&c.game);
        let visible: Vec<MatchEvent> = c.game.log.visible().into_iter().cloned().collect();
        let current = folded.states.last().map_or(1, |s| s.set);
        let in_set: Vec<&MatchEvent> = visible
            .iter()
            .zip(&folded.states)
            .filter(|(e, s)| s.set == set && e.kind() != EventKind::SetEnd)
            .map(|(e, _)| e)
            .collect();
        let Some(first) = in_set.first() else {
            if set == current {
                return self.set_lineup(team, court, setter, clock);
            }
            return Err(Error::BadLineup(format!("set {set} has no codes")));
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
        if let Err(e) = c.rotations_follow(team, set) {
            while c.game.log.len() > before {
                c.game.log.undo();
            }
            return Err(e.into());
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
            // A correction keeps the code's times: the touch happened then.
            let new = NewEvent {
                clock: None,
                video_ms: None,
                ..new
            };
            let id = log.correct(ids, target, new)?;
            let repair = |t: Touch| NewEvent::code(Code::Touch(t), Producer::Repair);
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

    /// Adds the codes on `text` just before an earlier code, read with the
    /// rally there as context. Codes going in before a rally's first code
    /// are a rally of their own: the console closes it as it would at the
    /// end of the match (the point, a side-out's rotation) and moves every
    /// later point code of the set on to its new score. A rotation after a
    /// missed side-out isn't carried into the codes after it; the checks
    /// show where they now disagree.
    pub fn insert_before(
        &mut self,
        id: &str,
        text: &str,
        clock: Option<ClockTime>,
    ) -> Result<Snapshot> {
        let c = self.current.as_mut().ok_or(Error::NoMatch)?;
        let season = self.season.as_mut().ok_or(Error::NoSeason)?;
        let target = ulid::Ulid::from_string(id)
            .map(EventId)
            .map_err(|_| Error::BadCode(format!("no event {id:?}")))?;
        let pos = c
            .position_of(target)
            .ok_or_else(|| Error::BadCode(format!("no event {id:?}")))?;
        let preview = c
            .preview_before(text.trim(), target)
            .ok_or_else(|| Error::BadCode(format!("no event {id:?}")))?;
        if !preview.ok {
            let why = preview
                .codes
                .iter()
                .find_map(|r| r.error.as_ref().map(|e| format!("{}: {e}", r.text)))
                .unwrap_or_else(|| "Type a code first.".into());
            return Err(Error::BadCode(why));
        }
        let codes: Vec<Code> = preview.codes.iter().map(|r| Code::parse(&r.text)).collect();
        let folded = fold(&c.game);
        let at = pos.checked_sub(1).and_then(|p| folded.states.get(p));
        let score_before = at.map(|s| s.score).unwrap_or_default();
        let setter_zones = at.map(|s| s.setter_position).unwrap_or_default();
        // A rally of its own: nothing of the rally at `target` comes first.
        let own_rally =
            c.touch_before(target).is_none() && codes.iter().any(|c| matches!(c, Code::Touch(_)));
        let new = |code: Code, producer| NewEvent {
            body: Body::Code(code),
            // Placed in the video where the code it goes before is.
            video_ms: c.game.log.visible().get(pos).and_then(|e| e.video_ms),
            clock,
            producer,
        };
        let mut events: Vec<NewEvent> = codes
            .iter()
            .map(|code| new(code.clone(), Producer::Keyboard))
            .collect();
        let typed_point = codes.iter().rev().find_map(|code| match code {
            Code::Point { team, .. } => Some(*team),
            _ => None,
        });
        let winner = typed_point.or_else(|| decided_by(&c.game, codes.iter()));
        if own_rally && winner.is_none() {
            return Err(Error::BadCode(
                "Who won this rally? End it with the winning code, or press Point for the winner."
                    .into(),
            ));
        }
        if own_rally && let Some(winner) = winner {
            if typed_point.is_none() {
                let (h, v) = match winner {
                    Team::Home => (score_before.home.saturating_add(1), score_before.visiting),
                    Team::Visiting => (score_before.home, score_before.visiting.saturating_add(1)),
                };
                let point = format!("{}p{h:02}:{v:02}", winner.as_char());
                events.push(new(Code::parse(&point), Producer::Repair));
            }
            let serving = codes.iter().find_map(|code| match code {
                Code::Touch(t) if t.skill == Skill::Serve => Some(t.team),
                _ => None,
            });
            if serving.is_some_and(|s| s != winner)
                && let Some(z) = *setter_zones.get(winner)
            {
                let next = if z == 1 { 6 } else { z - 1 };
                let rotation = format!("{}z{next}", winner.as_char());
                events.push(new(Code::parse(&rotation), Producer::Repair));
            }
        }
        let before = c.game.log.len();
        let change = || -> std::result::Result<(), LogError> {
            for e in events {
                c.game.log.insert_before(&mut c.ids, target, e)?;
            }
            if own_rally {
                c.rescore_from(pos, score_before)?;
            }
            c.reclose(clock)?;
            c.regreen(pos)?;
            c.drop_open_green()?;
            c.rerotate(pos)
        };
        if let Err(e) = change() {
            while c.game.log.len() > before {
                c.game.log.undo();
            }
            return Err(e.into());
        }
        c.persist(season, before)?;
        self.snapshot()
    }

    /// Writes the console's side-out rotations again from the rally at
    /// event `id` to the end of its set ("Fix rotations from here"): what
    /// an edit does by itself, for a match whose rotations went wrong
    /// before it did. The scout's own rotation codes stay.
    pub fn fix_rotations(&mut self, id: &str) -> Result<Snapshot> {
        let c = self.current.as_mut().ok_or(Error::NoMatch)?;
        let season = self.season.as_mut().ok_or(Error::NoSeason)?;
        let target = ulid::Ulid::from_string(id)
            .map(EventId)
            .map_err(|_| Error::BadCode(format!("no event {id:?}")))?;
        let pos = c
            .position_of(target)
            .ok_or_else(|| Error::BadCode(format!("no event {id:?}")))?;
        let before = c.game.log.len();
        if let Err(e) = c.rerotate(pos) {
            while c.game.log.len() > before {
                c.game.log.undo();
            }
            return Err(e.into());
        }
        if c.game.log.len() > before {
            c.persist(season, before)?;
            c.marks.push(before);
        }
        self.snapshot()
    }

    /// What [`Session::fix_rotations`] would do from event `id`, without
    /// doing it: how many codes it writes, the rallies they're in, and the
    /// issues before and after. A dry run on the match in memory, rolled
    /// back before returning; nothing is saved and undo is untouched.
    pub fn fix_rotations_preview(&mut self, id: &str) -> Result<FixPreview> {
        let issues_before = self.snapshot()?.issues.len();
        let target = ulid::Ulid::from_string(id)
            .map(EventId)
            .map_err(|_| Error::BadCode(format!("no event {id:?}")))?;
        let (before, redo, done) = {
            let c = self.current.as_mut().ok_or(Error::NoMatch)?;
            let pos = c
                .position_of(target)
                .ok_or_else(|| Error::BadCode(format!("no event {id:?}")))?;
            let before = c.game.log.len();
            let redo = c.redo.clone();
            (before, redo, c.rerotate(pos))
        };
        let result = match done {
            Err(e) => Err(e.into()),
            Ok(()) => self.snapshot().map(|after| {
                let added: Vec<String> = self
                    .current
                    .as_ref()
                    .map(|c| {
                        c.game.log.all()[before..]
                            .iter()
                            .map(|e| e.id.to_string())
                            .collect()
                    })
                    .unwrap_or_default();
                let rallies: Vec<u16> = after
                    .groups
                    .iter()
                    .filter(|g| g.rows.iter().any(|r| added.contains(&r.id)))
                    .filter_map(|g| g.rally)
                    .collect();
                FixPreview {
                    codes: added.len(),
                    from_rally: rallies.iter().min().copied(),
                    to_rally: rallies.iter().max().copied(),
                    issues_before,
                    issues_after: after.issues.len(),
                }
            }),
        };
        if let Some(c) = self.current.as_mut() {
            while c.game.log.len() > before {
                c.game.log.undo();
            }
            c.redo = redo;
        }
        result
    }

    /// [`Session::preview`] for codes going in just before an earlier code.
    pub fn preview_before(&self, text: &str, id: &str) -> Result<Preview> {
        let c = self.current.as_ref().ok_or(Error::NoMatch)?;
        let target = ulid::Ulid::from_string(id)
            .map(EventId)
            .map_err(|_| Error::BadCode(format!("no event {id:?}")))?;
        c.preview_before(text, target)
            .ok_or_else(|| Error::BadCode(format!("no event {id:?}")))
    }

    /// Marks where a play went on the court (GitHub #11), as positions in
    /// the play's frame (`vb_engine::court`: its team's half at the bottom).
    /// Without `start` the play starts in the middle of its start zone (a
    /// set or block is marked where it was); with neither, the marks are
    /// cleared. The mark writes its zones into the code, over typed ones
    /// ([`mark_zones`]); clearing it puts back the zones the code had
    /// before it was marked. A serve's marks go on the reception straight
    /// after it and an attack's on the dig, as `.dvw` keeps them (and the
    /// other way round). One change: one undo takes back the marks and
    /// the zones.
    pub fn set_points(
        &mut self,
        id: &str,
        start: Option<(f64, f64)>,
        end: Option<(f64, f64)>,
    ) -> Result<Snapshot> {
        let c = self.current.as_mut().ok_or(Error::NoMatch)?;
        let season = self.season.as_mut().ok_or(Error::NoSeason)?;
        let target = ulid::Ulid::from_string(id)
            .map(EventId)
            .map_err(|_| Error::BadCode(format!("no event {id:?}")))?;
        let touch = c
            .game
            .log
            .get(target)
            .and_then(touch_of)
            .ok_or_else(|| Error::BadCode("Only a touch has a direction.".into()))?;
        let cell = |(x, y): (f64, f64)| Some(vb_engine::court::index(x, y));
        let at_one_place = matches!(touch.skill, Skill::Set | Skill::Block);
        let points = match (start, end) {
            (None, None) => Points::default(),
            // A set or block is where it was: a click marks that place.
            (None, Some(end)) if at_one_place => Points {
                start: cell(end),
                mid: None,
                end: cell(end),
            },
            (start, Some(end)) => {
                let start = start.or_else(|| zone_start(&touch)).ok_or_else(|| {
                    Error::BadCode(format!(
                        "{touch} has no start zone: drag from where the play started."
                    ))
                })?;
                Points {
                    start: cell(start),
                    mid: None,
                    end: cell(end),
                }
            }
            (Some(start), None) => Points {
                start: cell(start),
                ..Points::default()
            },
        };
        let cleared = points == Points::default();
        let marked = |log: &EventLog, id: EventId, t: &Touch| {
            if cleared {
                unmarked_zones(log, id, t)
            } else {
                mark_zones(t, points, start.is_some() || at_one_place)
            }
        };
        let partner = pair_partner(&c.game.log, target).and_then(|(partner, _)| {
            let other = c.game.log.get(partner).and_then(touch_of)?;
            matches!(
                (touch.skill, other.skill),
                (Skill::Serve, Skill::Reception)
                    | (Skill::Reception, Skill::Serve)
                    | (Skill::Attack, Skill::Dig)
                    | (Skill::Dig, Skill::Attack)
            )
            .then_some((partner, other))
        });
        let before = c.game.log.len();
        let mut ids = Vec::new();
        let mut change = || -> std::result::Result<(), LogError> {
            let mut mark = |id: EventId, t: &Touch, producer| {
                let now = marked(&c.game.log, id, t);
                let same = c.game.log.get(id).is_some_and(|e| e.points == points);
                if now == *t && same {
                    return Ok(());
                }
                // A correction, so that undo takes the mark back.
                let new = c.game.log.correct(
                    &mut c.ids,
                    id,
                    NewEvent::code(Code::Touch(now), producer),
                )?;
                c.game.log.set_points(new, points)?;
                ids.push(new);
                Ok(())
            };
            mark(target, &touch, Producer::Tap)?;
            if let Some((partner, other)) = &partner {
                mark(*partner, other, Producer::Repair)?;
            }
            Ok(())
        };
        if let Err(e) = change() {
            while c.game.log.len() > before {
                c.game.log.undo();
            }
            return Err(e.into());
        }
        if c.game.log.len() > before {
            c.persist(season, before)?;
        }
        self.snapshot()
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
                // One code in, one code out: no set is added in front.
                let preview = readback::preview(
                    text,
                    &c.game,
                    c.names(),
                    before.as_ref(),
                    &c.defaults,
                    None,
                    None,
                );
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
        // A console-written code the scout edits is the scout's call: the
        // ending and the rotations aren't written again over it.
        let theirs = c
            .game
            .log
            .get(target)
            .is_some_and(|e| e.producer == Producer::Repair);
        let at = c.position_of(target).unwrap_or(0);
        let before = c.game.log.len();
        change(&mut c.game.log, &mut c.ids, target, new)?;
        let repair = if theirs {
            Ok(())
        } else {
            c.reclose(clock)
                .and_then(|()| c.regreen(at))
                .and_then(|()| c.drop_open_green())
                .and_then(|()| c.rerotate(at))
        };
        if let Err(e) = repair {
            while c.game.log.len() > before {
                c.game.log.undo();
            }
            return Err(e.into());
        }
        c.persist(season, before)?;
        self.snapshot()
    }

    /// Writes the open match as a `.dvw` file.
    /// The playlist `filter` picks from the open match (PRD F-23): its
    /// touches with a video time, each with the season's pre- and
    /// post-roll.
    pub fn clips(&self, filter: &ClipFilter) -> Result<Clips> {
        let c = self.current.as_ref().ok_or(Error::NoMatch)?;
        let roll = clips::Roll {
            pre_ms: u64::from(c.video.pre_roll_ms),
            post_ms: u64::from(c.video.post_roll_ms),
        };
        Ok(clips::build(&c.game, &c.teams, filter, roll))
    }

    /// The open match's codes a search in the code log finds, by event id
    /// in match order (`clips::find`).
    pub fn find_codes(&self, filter: &ClipFilter) -> Result<Vec<String>> {
        let c = self.current.as_ref().ok_or(Error::NoMatch)?;
        Ok(clips::find(&c.game, filter))
    }

    /// Remembers the video opened with the match (`None` forgets it).
    pub fn set_video(&mut self, path: Option<&str>) -> Result<Snapshot> {
        let c = self.current.as_mut().ok_or(Error::NoMatch)?;
        let season = self.season.as_mut().ok_or(Error::NoSeason)?;
        season.set_match_video(c.id, path)?;
        c.video_path = path.map(str::to_owned);
        self.snapshot()
    }

    /// Writes the match as a `.dvw`; the video's path goes into
    /// `[3VIDEO]` only `with_video` (it is personal data).
    pub fn export_dvw(&self, path: &Path, with_video: bool) -> Result<()> {
        let c = self.current.as_ref().ok_or(Error::NoMatch)?;
        let bytes = self.season()?.export_dvw(c.id, with_video)?;
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
