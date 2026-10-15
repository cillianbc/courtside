//! What the UI shows, built from one fold and one validation of the open
//! match. Serialised as camelCase JSON; `apps/desktop/src/lib/snapshot.ts`
//! mirrors these types.

use courtside_ffi::validate::{rule_name, severity_name};
use courtside_store::{MatchSummary, Team as StoreTeam};
use serde::Serialize;
use vb_engine::validate::{Severity, validate_folded};
use vb_engine::{EventKind, Match, MatchEvent, Producer, Sides, fold::fold_events};

use crate::readback;
use vb_grammar::{Code, Skill, Team};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Side {
    Home,
    Visiting,
}

impl From<Team> for Side {
    fn from(t: Team) -> Self {
        match t {
            Team::Home => Side::Home,
            Team::Visiting => Side::Visiting,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Pair<T> {
    pub home: T,
    pub visiting: T,
}

impl<T> Pair<T> {
    fn from_fn(mut f: impl FnMut(Team) -> T) -> Self {
        Pair {
            home: f(Team::Home),
            visiting: f(Team::Visiting),
        }
    }
}

impl<T: Clone> From<&Sides<T>> for Pair<T> {
    fn from(s: &Sides<T>) -> Self {
        Pair {
            home: s.home.clone(),
            visiting: s.visiting.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    pub match_id: String,
    pub teams: Pair<TeamView>,
    /// The set in progress, from 1.
    pub set: u8,
    pub score: Pair<u8>,
    /// The team serving the open rally (or the next one), when known.
    pub serving: Option<Side>,
    pub sets: Vec<SetView>,
    /// Shirt numbers in zones 1–6.
    pub court: Pair<[Option<u8>; 6]>,
    pub setter: Pair<Option<u8>>,
    pub setter_zone: Pair<Option<u8>>,
    /// Used in the current set.
    pub timeouts: Pair<u8>,
    pub substitutions: Pair<u8>,
    /// The code stream, split into rallies; the last group may be open.
    pub groups: Vec<Group>,
    pub issues: Vec<IssueView>,
    /// Events in the log, saved or not.
    pub events: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TeamView {
    pub name: String,
    pub players: Vec<PlayerView>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PlayerView {
    pub number: u8,
    pub name: String,
    pub libero: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SetView {
    pub number: u8,
    pub score: Pair<u8>,
    pub finished: bool,
}

/// Consecutive codes ending at a point code (a closed rally), a set end,
/// or the end of the log (the open rally).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Group {
    pub set: u8,
    /// Rally number within the set, from 1, for groups that hold a rally.
    pub rally: Option<u16>,
    pub serving: Option<Side>,
    pub won_by: Option<Side>,
    pub score_after: Option<Pair<u8>>,
    /// Ends with a point code.
    pub closed: bool,
    pub rows: Vec<Row>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Row {
    pub id: String,
    /// Index into the visible events.
    pub index: usize,
    pub text: String,
    pub team: Option<Side>,
    pub kind: &'static str,
    /// The read-back, as in the command bar.
    pub meaning: String,
    /// Written by the console (a point code or rotation), not typed.
    pub auto: bool,
    /// The worst issue on this code.
    pub severity: Option<&'static str>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IssueView {
    pub severity: &'static str,
    pub rule: &'static str,
    pub message: String,
    pub event_id: String,
    pub index: usize,
    pub knock_on: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MatchItem {
    pub id: String,
    pub home: String,
    pub visiting: String,
    pub date: Option<String>,
    pub source_name: Option<String>,
    pub events: usize,
}

impl From<MatchSummary> for MatchItem {
    fn from(m: MatchSummary) -> Self {
        MatchItem {
            id: m.id.to_string(),
            home: m.home,
            visiting: m.visiting,
            date: m.date,
            source_name: m.source_name,
            events: m.events,
        }
    }
}

fn kind_name(k: EventKind) -> &'static str {
    match k {
        EventKind::Touch => "touch",
        EventKind::Point => "point",
        EventKind::Lineup => "lineup",
        EventKind::SetterChange => "setterChange",
        EventKind::SetterPosition => "setterPosition",
        EventKind::Substitution => "substitution",
        EventKind::Timeout => "timeout",
        EventKind::SetEnd => "setEnd",
        EventKind::Comment => "comment",
        EventKind::Removed => "removed",
        EventKind::Unrecognised => "unrecognised",
    }
}

fn team_view(t: &StoreTeam) -> TeamView {
    TeamView {
        name: t.name.clone(),
        players: t
            .players
            .iter()
            .map(|p| PlayerView {
                number: p.number,
                name: [p.first_name.as_str(), p.last_name.as_str()]
                    .iter()
                    .filter(|s| !s.is_empty())
                    .copied()
                    .collect::<Vec<_>>()
                    .join(" "),
                libero: p.libero,
            })
            .collect(),
    }
}

fn is_serve(e: &MatchEvent) -> Option<Team> {
    match e.code() {
        Some(Code::Touch(t)) if t.skill == Skill::Serve => Some(t.team),
        _ => None,
    }
}

pub fn build(match_id: &str, game: &Match, teams: &(StoreTeam, StoreTeam)) -> Snapshot {
    let names = (teams.0.name.as_str(), teams.1.name.as_str());
    let events = game.log.visible();
    let folded = fold_events(game, &events);
    let issues = validate_folded(game, &events, &folded);

    let mut worst: Vec<Option<Severity>> = vec![None; events.len()];
    for i in &issues {
        if let Some(w) = worst.get_mut(i.index) {
            *w = (*w).max(Some(i.severity));
        }
    }

    let last = folded.states.last();
    let set = last.map_or(1, |s| s.set);
    let score = last.map_or(
        Pair {
            home: 0,
            visiting: 0,
        },
        |s| Pair {
            home: s.score.home,
            visiting: s.score.visiting,
        },
    );

    // The rally each point code ends. The fold also counts the open rally
    // at the end of the log, which has no point code yet.
    let mut rally_end = vec![None; events.len()];
    for (n, r) in folded.rallies.iter().enumerate() {
        let is_point = events
            .get(r.last)
            .is_some_and(|e| e.kind() == EventKind::Point);
        if let (true, Some(slot)) = (is_point, rally_end.get_mut(r.last)) {
            *slot = Some(n);
        }
    }

    let mut groups: Vec<Group> = Vec::new();
    let mut rows: Vec<Row> = Vec::new();
    let mut set_rally: (u8, u16) = (0, 0);
    let mut group_set = 1;
    for (i, e) in events.iter().enumerate() {
        let state = folded.states.get(i);
        if rows.is_empty() {
            group_set = state.map_or(1, |s| s.set);
        }
        let kind = e.kind();
        rows.push(Row {
            id: e.id.0.to_string(),
            index: i,
            text: match &e.body {
                vb_engine::Body::Code(c) => c.to_string(),
                vb_engine::Body::Comment(t) => t.clone(),
                vb_engine::Body::Removed => String::new(),
            },
            team: e.code().and_then(Code::team).map(Side::from),
            kind: kind_name(kind),
            meaning: match &e.body {
                vb_engine::Body::Code(c) => readback::read(&c.to_string(), game, names).meaning,
                _ => String::new(),
            },
            auto: e.producer == Producer::Repair,
            severity: worst.get(i).copied().flatten().map(severity_name),
        });
        let ended = rally_end.get(i).copied().flatten();
        if ended.is_some() || kind == EventKind::SetEnd {
            let rally = ended.and_then(|n| folded.rallies.get(n));
            let number = rally.map(|r| {
                if set_rally.0 != r.set {
                    set_rally = (r.set, 0);
                }
                set_rally.1 += 1;
                set_rally.1
            });
            groups.push(Group {
                set: group_set,
                rally: number,
                serving: rally.and_then(|r| r.serving).map(Side::from),
                won_by: rally.and_then(|r| r.won_by).map(Side::from),
                score_after: rally.map(|r| Pair {
                    home: r.score_after.home,
                    visiting: r.score_after.visiting,
                }),
                closed: rally.is_some(),
                rows: std::mem::take(&mut rows),
            });
        }
    }

    // The open rally: who serves it, from its serve or the last point.
    let open_start = events.len() - rows.len();
    let serving = events
        .get(open_start..)
        .and_then(|open| open.iter().find_map(|e| is_serve(e)))
        .or_else(|| {
            folded
                .rallies
                .iter()
                .rev()
                .filter(|r| r.won_by.is_some())
                .take_while(|r| r.set == set)
                .find_map(|r| r.won_by)
        })
        .map(Side::from);
    let open_number = if set_rally.0 == set {
        set_rally.1 + 1
    } else {
        1
    };
    groups.push(Group {
        set,
        rally: Some(open_number),
        serving,
        won_by: None,
        score_after: None,
        closed: false,
        rows,
    });

    let mut sets: Vec<SetView> = Vec::new();
    for r in folded.rallies.iter().filter(|r| r.won_by.is_some()) {
        let s = Pair {
            home: r.score_after.home,
            visiting: r.score_after.visiting,
        };
        match sets.last_mut() {
            Some(v) if v.number == r.set => v.score = s,
            _ => sets.push(SetView {
                number: r.set,
                score: s,
                finished: false,
            }),
        }
    }
    for v in &mut sets {
        v.finished = v.number < set;
    }
    if sets.last().is_none_or(|v| v.number < set) {
        sets.push(SetView {
            number: set,
            score: score.clone(),
            finished: false,
        });
    }

    let count = |kind: EventKind| {
        Pair::from_fn(|team| {
            let n = events
                .iter()
                .zip(&folded.states)
                .filter(|(e, s)| {
                    s.set == set && e.kind() == kind && e.code().and_then(Code::team) == Some(team)
                })
                .count();
            u8::try_from(n).unwrap_or(u8::MAX)
        })
    };

    Snapshot {
        match_id: match_id.to_owned(),
        teams: Pair {
            home: team_view(&teams.0),
            visiting: team_view(&teams.1),
        },
        set,
        score,
        serving,
        sets,
        court: last.map_or(
            Pair {
                home: [None; 6],
                visiting: [None; 6],
            },
            |s| Pair::from(&s.court),
        ),
        setter: last.map_or(
            Pair {
                home: None,
                visiting: None,
            },
            |s| Pair::from(&s.setter),
        ),
        setter_zone: last.map_or(
            Pair {
                home: None,
                visiting: None,
            },
            |s| Pair::from(&s.setter_position),
        ),
        timeouts: count(EventKind::Timeout),
        substitutions: count(EventKind::Substitution),
        groups,
        issues: issues
            .iter()
            .map(|i| IssueView {
                severity: severity_name(i.severity),
                rule: rule_name(i.rule),
                message: i.message.clone(),
                event_id: i.event.0.to_string(),
                index: i.index,
                knock_on: i.cause.is_some(),
            })
            .collect(),
        events: game.log.len(),
    }
}
