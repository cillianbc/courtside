//! The six validation rules (PRD domain model), run over the visible events
//! and the fold. Nothing here changes the match; it only reports.

use std::collections::{HashMap, HashSet};

use vb_grammar::{Code, Player, Skill, Team};

use crate::event::{EventId, EventKind, MatchEvent};
use crate::fold::{Folded, Score, State, fold_events};
use crate::model::{Match, Outcome};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Severity {
    Warning,
    Error,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Rule {
    /// 1. Every point advances one side by one; sets end at a won score.
    Score,
    /// 2. Side-outs rotate, the rally winner serves, players are on court.
    Rotation,
    /// 3. Back-row players don't attack from the front zone or block.
    BackRow,
    /// 4. Substitutions: on and off court, six per set, re-entry, libero.
    Substitution,
    /// 5. A serve is followed by the other team's reception, or ends the
    ///    rally; the point goes to the team the touches say won it.
    ServeReception,
    /// 6. Unknown codes, players not on the team sheet, bad zones.
    Code,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Issue {
    pub severity: Severity,
    pub rule: Rule,
    pub event: EventId,
    /// Index into the visible events.
    pub index: usize,
    pub message: String,
}

fn side(team: Team) -> &'static str {
    match team {
        Team::Home => "home",
        Team::Visiting => "visiting",
    }
}

fn other(team: Team) -> Team {
    match team {
        Team::Home => Team::Visiting,
        Team::Visiting => Team::Home,
    }
}

fn num(d: &vb_grammar::code::Digits) -> Option<u8> {
    d.value().and_then(|v| u8::try_from(v).ok())
}

fn touch_player(code: &Code) -> Option<(Team, u8)> {
    match code {
        Code::Touch(t) if t.skill != Skill::Green => match &t.player {
            Player::Number(d) => num(d).map(|n| (t.team, n)),
            Player::Unknown => None,
        },
        _ => None,
    }
}

/// Zone 1–6 of `n` on `team`'s court, if on court.
fn zone_of(st: &State, team: Team, n: u8) -> Option<usize> {
    st.court
        .get(team)
        .iter()
        .position(|p| *p == Some(n))
        .map(|i| i + 1)
}

/// The setter position one rotation on from `p` (1 → 6 → 5 …).
fn rotated(p: u8) -> u8 {
    if p == 1 { 6 } else { p - 1 }
}

struct Validator<'a> {
    m: &'a Match,
    events: &'a [&'a MatchEvent],
    f: &'a Folded,
    issues: Vec<Issue>,
}

impl Validator<'_> {
    fn push(&mut self, i: usize, severity: Severity, rule: Rule, message: String) {
        self.issues.push(Issue {
            severity,
            rule,
            event: self.events[i].id,
            index: i,
            message,
        });
    }

    /// Whether event `i` is the first of a set (its lineup loads there).
    fn starts_set(&self, i: usize) -> bool {
        self.events[..i]
            .iter()
            .rev()
            .find(|e| e.kind() != EventKind::Comment)
            .is_none_or(|e| e.kind() == EventKind::SetEnd)
    }

    /// Court before event `i`.
    fn court_before(&self, i: usize, team: Team) -> [Option<u8>; 6] {
        if self.starts_set(i) {
            let set = self.f.states[i].set;
            match team {
                Team::Home => self.m.teams.home.starting_lineup(set),
                Team::Visiting => self.m.teams.visiting.starting_lineup(set),
            }
        } else {
            *self.f.states[i - 1].court.get(team)
        }
    }

    fn score_before(&self, i: usize) -> Score {
        if self.starts_set(i) {
            Score::default()
        } else {
            self.f.states[i - 1].score
        }
    }

    fn won(&self, set: u8, s: Score) -> bool {
        let target = self.m.regulation.set_target(set);
        let (hi, lo) = (s.home.max(s.visiting), s.home.min(s.visiting));
        hi >= target && hi - lo >= 2 && (hi == target || hi - lo == 2)
    }

    fn codes(&mut self) {
        for i in 0..self.events.len() {
            let Some(code) = self.events[i].code() else {
                continue;
            };
            let mut found: Vec<(Severity, String)> = Vec::new();
            match code {
                Code::Unrecognised(s) if !s.is_empty() => {
                    found.push((Severity::Warning, format!("unrecognised code {s:?}")));
                }
                Code::Touch(t) => {
                    if let Some((team, n)) = touch_player(code)
                        && self.m.teams.get(team).player(n).is_none()
                    {
                        found.push((
                            Severity::Error,
                            format!("player {n} is not on the {} team sheet", side(team)),
                        ));
                    }
                    if t.skill != Skill::Green {
                        if let Some(e) = t.evaluation().filter(|e| !"#+!-/=".contains(*e)) {
                            found.push((Severity::Error, format!("unknown evaluation {e:?}")));
                        }
                        if let Some(k) = t.skill_type().filter(|k| !"HMQTUNO".contains(*k)) {
                            found.push((Severity::Warning, format!("unknown skill type {k:?}")));
                        }
                    }
                    for (what, z) in [("start zone", t.start_zone()), ("end zone", t.end_zone())] {
                        if let Some(z) = z.filter(|z| !('1'..='9').contains(z)) {
                            found.push((Severity::Error, format!("{what} {z:?} is not 1–9")));
                        }
                    }
                    if let Some(z) = t.end_subzone().filter(|z| !('A'..='D').contains(z)) {
                        found.push((Severity::Error, format!("end subzone {z:?} is not A–D")));
                    }
                }
                Code::SetterPosition { position, .. }
                    if !num(position).is_some_and(|p| (1..=6).contains(&p)) =>
                {
                    found.push((
                        Severity::Error,
                        format!("setter position {position} is not 1–6"),
                    ));
                }
                Code::Point { home, visiting, .. }
                    if num(home).is_none() || num(visiting).is_none() =>
                {
                    found.push((
                        Severity::Error,
                        "point code without a readable score".into(),
                    ));
                }
                Code::SetEnd { set, .. } => {
                    let ending = self.f.states[i].set.saturating_sub(1);
                    if num(set) != Some(ending) {
                        found.push((
                            Severity::Error,
                            format!("set end {set} where set {ending} is being played"),
                        ));
                    }
                }
                _ => {}
            }
            for (severity, message) in found {
                self.push(i, severity, Rule::Code, message);
            }
        }
    }

    fn score(&mut self) {
        for i in 0..self.events.len() {
            let st = &self.f.states[i];
            match self.events[i].code() {
                Some(Code::Point { team, .. }) => {
                    let before = self.score_before(i);
                    if self.won(st.set, before) {
                        self.push(
                            i,
                            Severity::Error,
                            Rule::Score,
                            format!(
                                "point after the set was won at {}:{}; missing set end?",
                                before.home, before.visiting
                            ),
                        );
                        continue;
                    }
                    let mut expected = before;
                    match team {
                        Team::Home => expected.home += 1,
                        Team::Visiting => expected.visiting += 1,
                    }
                    if st.score != expected {
                        self.push(
                            i,
                            Severity::Error,
                            Rule::Score,
                            format!(
                                "point to {}: score {}:{} → {}:{}, expected {}:{}",
                                side(*team),
                                before.home,
                                before.visiting,
                                st.score.home,
                                st.score.visiting,
                                expected.home,
                                expected.visiting
                            ),
                        );
                    }
                }
                Some(Code::SetEnd { .. }) => {
                    let set = st.set.saturating_sub(1);
                    if !self.won(set, st.score) {
                        self.push(
                            i,
                            Severity::Error,
                            Rule::Score,
                            format!(
                                "set {set} ends at {}:{}, which doesn't win it",
                                st.score.home, st.score.visiting
                            ),
                        );
                    }
                }
                _ => {}
            }
        }
    }

    fn first_serve(&self, r: usize) -> Option<usize> {
        let rally = &self.f.rallies[r];
        (rally.first..=rally.last).find(
            |&i| matches!(self.events[i].code(), Some(Code::Touch(t)) if t.skill == Skill::Serve),
        )
    }

    fn rotation(&mut self) {
        let rallies = self.f.rallies.clone();
        for (k, pair) in rallies.windows(2).enumerate() {
            let (r, next) = (&pair[0], &pair[1]);
            let (Some(winner), Some(server)) = (r.won_by, r.serving) else {
                continue;
            };
            if next.set != r.set {
                continue;
            }
            let at = self.first_serve(k + 1);
            if let Some(s) = at
                && next.serving != Some(winner)
            {
                self.push(
                    s,
                    Severity::Error,
                    Rule::Rotation,
                    format!(
                        "{} serves but {} won the last rally",
                        side(other(winner)),
                        side(winner)
                    ),
                );
            }
            let check_at = at.unwrap_or(next.last);
            let end = &self.f.states[r.last];
            let then = &self.f.states[check_at];
            for team in [Team::Home, Team::Visiting] {
                let (Some(p), Some(q)) = (
                    *end.setter_position.get(team),
                    *then.setter_position.get(team),
                ) else {
                    continue;
                };
                let rotates = team == winner && winner != server;
                let expected = if rotates { rotated(p) } else { p };
                if q == expected {
                    continue;
                }
                let (severity, message) = if rotates && q == p {
                    (
                        Severity::Error,
                        format!("{} won a side-out but didn't rotate", side(team)),
                    )
                } else {
                    (
                        Severity::Warning,
                        format!(
                            "{} setter moved from zone {p} to {q}; expected {expected}",
                            side(team)
                        ),
                    )
                };
                self.push(check_at, severity, Rule::Rotation, message);
            }
        }
        for i in 0..self.events.len() {
            let Some(code) = self.events[i].code() else {
                continue;
            };
            let Some((team, n)) = touch_player(code) else {
                continue;
            };
            let st = &self.f.states[i];
            if self.m.teams.get(team).is_libero(n) || st.court.get(team).iter().all(Option::is_none)
            {
                continue;
            }
            let is_serve = matches!(code, Code::Touch(t) if t.skill == Skill::Serve);
            match zone_of(st, team, n) {
                None => self.push(
                    i,
                    Severity::Warning,
                    Rule::Rotation,
                    format!("{} player {n} is not on court", side(team)),
                ),
                Some(z) if is_serve && z != 1 => self.push(
                    i,
                    Severity::Warning,
                    Rule::Rotation,
                    format!("{} server {n} is in zone {z}, not zone 1", side(team)),
                ),
                _ => {}
            }
        }
    }

    fn back_row(&mut self) {
        for i in 0..self.events.len() {
            let Some(Code::Touch(t)) = self.events[i].code() else {
                continue;
            };
            let Some((team, n)) = touch_player(&Code::Touch(t.clone())) else {
                continue;
            };
            let libero = self.m.teams.get(team).is_libero(n);
            let zone = zone_of(&self.f.states[i], team, n);
            let back = zone.is_some_and(|z| matches!(z, 1 | 5 | 6));
            match t.skill {
                Skill::Block if libero => self.push(
                    i,
                    Severity::Error,
                    Rule::BackRow,
                    format!("libero {n} blocked"),
                ),
                Skill::Block if back => self.push(
                    i,
                    Severity::Error,
                    Rule::BackRow,
                    format!("back-row player {n} (zone {}) blocked", zone.unwrap_or(0)),
                ),
                Skill::Attack if back && matches!(t.start_zone(), Some('2' | '3' | '4')) => self
                    .push(
                        i,
                        Severity::Warning,
                        Rule::BackRow,
                        format!(
                            "back-row player {n} (zone {}) attacked from zone {}",
                            zone.unwrap_or(0),
                            t.start_zone().unwrap_or(' ')
                        ),
                    ),
                _ => {}
            }
        }
    }

    fn substitutions(&mut self) {
        #[derive(Default)]
        struct Book {
            count: u8,
            pairs: HashMap<u8, u8>,
            closed: HashSet<u8>,
        }
        let mut books: HashMap<(u8, Team), Book> = HashMap::new();
        for i in 0..self.events.len() {
            let Some(Code::Substitution {
                team, out, r#in, ..
            }) = self.events[i].code()
            else {
                continue;
            };
            let team = *team;
            let (Some(out), Some(inn)) = (num(out), num(r#in)) else {
                self.push(
                    i,
                    Severity::Error,
                    Rule::Substitution,
                    "substitution without readable shirt numbers".into(),
                );
                continue;
            };
            let sheet = self.m.teams.get(team);
            let court = self.court_before(i, team);
            let mut found: Vec<(Severity, String)> = Vec::new();
            if out == inn {
                found.push((
                    Severity::Error,
                    format!("player {out} replaced by themselves"),
                ));
            }
            if sheet.is_libero(out) || sheet.is_libero(inn) {
                found.push((
                    Severity::Warning,
                    "libero replacements are not substitutions".into(),
                ));
            }
            if sheet.player(inn).is_none() {
                found.push((
                    Severity::Error,
                    format!("player {inn} is not on the {} team sheet", side(team)),
                ));
            }
            if !court.contains(&Some(out)) && court.iter().any(Option::is_some) {
                found.push((Severity::Error, format!("player {out} is not on court")));
            }
            if court.contains(&Some(inn)) {
                found.push((Severity::Error, format!("player {inn} is already on court")));
            }
            let book = books.entry((self.f.states[i].set, team)).or_default();
            book.count += 1;
            if book.count == 7 {
                found.push((
                    Severity::Error,
                    format!("seventh {} substitution in the set", side(team)),
                ));
            }
            if let Some(p) = [out, inn].into_iter().find(|p| book.closed.contains(p)) {
                found.push((
                    Severity::Error,
                    format!("player {p} has already gone off and come back on this set"),
                ));
            } else if let Some(&partner) = book.pairs.get(&out) {
                if partner == inn {
                    book.closed.insert(out);
                    book.closed.insert(inn);
                } else {
                    found.push((
                        Severity::Error,
                        format!("player {out} can only be replaced by {partner}"),
                    ));
                }
            } else if let Some(&partner) = book.pairs.get(&inn) {
                found.push((
                    Severity::Error,
                    format!("player {inn} can only come on for {partner}"),
                ));
            } else {
                book.pairs.insert(out, inn);
                book.pairs.insert(inn, out);
            }
            for (severity, message) in found {
                self.push(i, severity, Rule::Substitution, message);
            }
        }
    }

    fn serve_reception(&mut self) {
        let last_rally = self.f.rallies.len().saturating_sub(1);
        for (k, r) in self.f.rallies.clone().iter().enumerate() {
            let touches: Vec<(usize, vb_grammar::Touch)> = (r.first..=r.last)
                .filter_map(|i| match self.events[i].code() {
                    Some(Code::Touch(t)) if t.skill != Skill::Green => Some((i, t.clone())),
                    _ => None,
                })
                .collect();
            if touches.is_empty() {
                continue;
            }
            if r.won_by.is_none() && k != last_rally {
                self.push(
                    r.last,
                    Severity::Error,
                    Rule::ServeReception,
                    "rally has no point code".into(),
                );
            }
            let mut serves = 0;
            for (j, (i, t)) in touches.iter().enumerate() {
                let next = touches.get(j + 1);
                let outcome = t
                    .evaluation()
                    .and_then(|e| self.m.winning_symbols.outcome(t.skill, e));
                match t.skill {
                    Skill::Serve => {
                        serves += 1;
                        if serves == 2 {
                            self.push(
                                *i,
                                Severity::Error,
                                Rule::ServeReception,
                                "second serve in one rally; missing point code before it?".into(),
                            );
                        }
                        if outcome.is_none()
                            && let Some((ni, n)) = next
                            && !(n.skill == Skill::Reception && n.team != t.team)
                        {
                            self.push(
                                *ni,
                                Severity::Warning,
                                Rule::ServeReception,
                                "a serve should be followed by the other team's reception".into(),
                            );
                        }
                    }
                    Skill::Reception => {
                        let prev = j.checked_sub(1).and_then(|p| touches.get(p));
                        if !prev.is_some_and(|(_, p)| p.skill == Skill::Serve && p.team != t.team) {
                            self.push(
                                *i,
                                Severity::Warning,
                                Rule::ServeReception,
                                "reception without a serve from the other team".into(),
                            );
                        }
                    }
                    _ => {}
                }
                // DataVolley records a decided rally as a pair: the touch
                // and the other team's matching one (ace then reception
                // error, blocked attack then block point).
                let winner = |t: &vb_grammar::Touch| {
                    let o = self.m.winning_symbols.outcome(t.skill, t.evaluation()?)?;
                    Some(if o == Outcome::Win {
                        t.team
                    } else {
                        other(t.team)
                    })
                };
                let paired = next.is_some_and(|(_, n)| {
                    n.team != t.team && winner(n).is_some() && winner(n) == winner(t)
                });
                if outcome.is_some()
                    && !paired
                    && let Some((ni, _)) = next
                {
                    self.push(
                        *ni,
                        Severity::Warning,
                        Rule::ServeReception,
                        format!(
                            "the rally continues after {} ended it",
                            Code::Touch(t.clone())
                        ),
                    );
                }
            }
            // The last touch that decides a rally says who won it.
            let decided = touches.iter().rev().find_map(|(i, t)| {
                let o = self.m.winning_symbols.outcome(t.skill, t.evaluation()?)?;
                Some((
                    *i,
                    t,
                    if o == Outcome::Win {
                        t.team
                    } else {
                        other(t.team)
                    },
                ))
            });
            if let (Some(won), Some((_, t, expected))) = (r.won_by, decided)
                && won != expected
            {
                self.push(
                    r.last,
                    Severity::Error,
                    Rule::ServeReception,
                    format!(
                        "point to {} but {} means {} won the rally",
                        side(won),
                        Code::Touch(t.clone()),
                        side(expected)
                    ),
                );
            }
        }
    }
}

/// Every issue in the match, in event order.
pub fn validate(m: &Match) -> Vec<Issue> {
    let events = m.log.visible();
    let folded = fold_events(m, &events);
    validate_folded(m, &events, &folded)
}

/// As [`validate`], for events already folded.
pub fn validate_folded(m: &Match, events: &[&MatchEvent], folded: &Folded) -> Vec<Issue> {
    let mut v = Validator {
        m,
        events,
        f: folded,
        issues: Vec::new(),
    };
    v.codes();
    v.score();
    v.rotation();
    v.back_row();
    v.substitutions();
    v.serve_reception();
    let mut issues = v.issues;
    issues.sort_by_key(|i| (i.index, std::cmp::Reverse(i.severity), i.rule));
    issues
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::{IdGen, NewEvent, Producer};
    use crate::model::{RosterPlayer, Sides, StartingZone, TeamSheet};

    fn sheet(lineup: [u8; 6], bench: &[u8], libero: Option<u8>) -> TeamSheet {
        let mut players: Vec<RosterPlayer> = lineup
            .iter()
            .enumerate()
            .map(|(i, n)| RosterPlayer {
                number: *n,
                starting_zones: [StartingZone::Zone(i as u8 + 1); 5],
                libero: false,
            })
            .collect();
        for n in bench.iter().copied().chain(libero) {
            players.push(RosterPlayer {
                number: n,
                starting_zones: [StartingZone::None; 5],
                libero: Some(n) == libero,
            });
        }
        TeamSheet { players }
    }

    fn check(codes: &[&str]) -> Vec<String> {
        let mut m = Match {
            teams: Sides {
                home: sheet([8, 12, 13, 11, 3, 14], &[5, 6, 7, 9, 10, 15, 16], Some(1)),
                visiting: sheet([16, 11, 18, 6, 10, 13], &[2, 3], None),
            },
            ..Match::default()
        };
        let mut ids = IdGen::new(0);
        for c in codes {
            m.log
                .append(&mut ids, NewEvent::code(Code::parse(c), Producer::Keyboard));
        }
        validate(&m)
            .iter()
            .map(|i| format!("{} {:?}: {}", i.index, i.rule, i.message))
            .collect()
    }

    const LINEUP: [&str; 4] = ["*P08>LUp", "*z1>LUp", "aP11>LUp", "az2>LUp"];

    fn with_lineup(codes: &[&str]) -> Vec<String> {
        let all: Vec<&str> = LINEUP.iter().chain(codes).copied().collect();
        check(&all)
    }

    #[test]
    fn clean_rallies() {
        assert_eq!(
            with_lineup(&[
                "*08SH-", "a16RH#", "a18AH#", "ap00:01", // side-out
                "az1", "a11SH=", "*p01:01", // serve error
                "*z6", "*12SH+", "a18RH-", "a11EH+", "a10AH-", "*13BH#", "*p02:01",
            ]),
            Vec::<String>::new()
        );
    }

    #[test]
    fn score_and_rotation() {
        assert_eq!(
            with_lineup(&["*08SH-", "a16RH#", "a18AH#", "ap00:02", "*12SH-"]),
            [
                "7 Score: point to visiting: score 0:0 → 0:2, expected 0:1",
                "8 Rotation: home serves but visiting won the last rally",
                "8 Rotation: visiting won a side-out but didn't rotate",
                "8 Rotation: home server 12 is in zone 2, not zone 1",
            ]
        );
    }

    #[test]
    fn point_against_the_touches() {
        assert_eq!(
            with_lineup(&["*08SH#", "ap00:01"]),
            ["5 ServeReception: point to visiting but *08SH# means home won the rally"]
        );
    }

    #[test]
    fn substitutions_and_back_row() {
        assert_eq!(
            with_lineup(&[
                "*c13:05", "*c05:07", "*c11:13", "*c01:12", "*08SH-", "a16RH-", "a18AH+", "*01BH-",
                "*08DH+",
            ]),
            [
                "5 Substitution: player 5 can only be replaced by 13",
                "6 Substitution: player 13 can only come on for 5",
                "7 Substitution: player 1 is not on court",
                "7 Substitution: player 12 is already on court",
                "7 Substitution: libero replacements are not substitutions",
                "11 BackRow: libero 1 blocked",
            ]
        );
    }

    #[test]
    fn serve_reception_and_codes() {
        assert_eq!(
            with_lineup(&[
                "*08SH-", "*12AH+", "a16RH#", "*44SH-", "a18AQ#", "ap00:01", "%%"
            ]),
            [
                "5 ServeReception: a serve should be followed by the other team's reception",
                "6 ServeReception: reception without a serve from the other team",
                "7 ServeReception: second serve in one rally; missing point code before it?",
                "7 Code: player 44 is not on the home team sheet",
                "7 Rotation: home player 44 is not on court",
                "8 ServeReception: a serve should be followed by the other team's reception",
                "10 Rotation: visiting won a side-out but didn't rotate",
                "10 Code: unrecognised code \"%%\"",
            ]
        );
    }
}
