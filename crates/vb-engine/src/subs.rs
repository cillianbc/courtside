//! Regular substitutions per team and set (FIVB rule 15.6): a starter may
//! leave and come back once, only in place of the player who replaced
//! them; that substitute may leave only for the same starter. Once a pair
//! has gone out and back, neither player can be changed again in the set.
//! Exceptional substitutions (an injured player) are outside this rule.
//!
//! The validator reports breaches from it; the console offers only the
//! legal players from it.

use std::collections::HashMap;

use vb_grammar::{Code, Team};

use crate::event::MatchEvent;
use crate::fold::Folded;

/// Where a starter and their substitute are in the set.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PairState {
    /// The starter went off and the substitute is on.
    Out,
    /// The starter came back for the substitute: both are used up.
    Back,
}

/// A starter and the substitute who replaced them in one set.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SubPair {
    /// The player who went off first.
    pub starter: u8,
    pub substitute: u8,
    pub state: PairState,
}

impl SubPair {
    fn has(&self, n: u8) -> bool {
        self.starter == n || self.substitute == n
    }

    fn partner(&self, n: u8) -> u8 {
        if self.starter == n {
            self.substitute
        } else {
            self.starter
        }
    }
}

/// A substitution against the regular rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Breach {
    /// The player's pair has already gone out and back this set.
    ReEntry(u8),
    /// The player going off can only be replaced by `partner`.
    OffForOther { off: u8, partner: u8 },
    /// The player coming on can only come on for `partner`.
    OnForOther { on: u8, partner: u8 },
}

/// One team's substitutions in one set.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SubBook {
    /// Substitution codes in the set, legal or not.
    pub count: u8,
    /// In the order the starters first went off.
    pub pairs: Vec<SubPair>,
}

/// Who may come on for one player on court.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubOptions {
    /// Bench players a regular substitution may bring on, in shirt order.
    pub on: Vec<u8>,
    /// Why nobody may, when `on` is empty.
    pub reason: Option<String>,
}

impl SubBook {
    fn pair_of(&self, n: u8) -> Option<&SubPair> {
        self.pairs.iter().find(|p| p.has(n))
    }

    /// Records `off` replaced by `on`; the breach, if it was against the
    /// rule (then the pairs are left as they were).
    pub fn record(&mut self, off: u8, on: u8) -> Option<Breach> {
        self.count = self.count.saturating_add(1);
        if let Some(p) = [off, on]
            .into_iter()
            .find(|&n| self.pair_of(n).is_some_and(|p| p.state == PairState::Back))
        {
            return Some(Breach::ReEntry(p));
        }
        if let Some(pair) = self.pairs.iter_mut().find(|p| p.has(off)) {
            let partner = pair.partner(off);
            if partner == on {
                pair.state = PairState::Back;
                return None;
            }
            return Some(Breach::OffForOther { off, partner });
        }
        if let Some(pair) = self.pair_of(on) {
            return Some(Breach::OnForOther {
                on,
                partner: pair.partner(on),
            });
        }
        self.pairs.push(SubPair {
            starter: off,
            substitute: on,
            state: PairState::Out,
        });
        None
    }

    /// Who from `bench` (players off court who aren't liberos) a regular
    /// substitution may bring on for `off`.
    pub fn options(&self, off: u8, bench: &[u8]) -> SubOptions {
        let mut bench: Vec<u8> = bench.to_vec();
        bench.sort_unstable();
        bench.dedup();
        let none = |reason: String| SubOptions {
            on: Vec::new(),
            reason: Some(reason),
        };
        match self.pair_of(off) {
            Some(p) if p.state == PairState::Back => none(format!(
                "#{} and #{} have gone off and back this set; only an exceptional substitution can change them.",
                p.starter, p.substitute
            )),
            Some(p) => {
                let partner = p.partner(off);
                if bench.contains(&partner) {
                    SubOptions {
                        on: vec![partner],
                        reason: None,
                    }
                } else {
                    none(format!(
                        "#{off} can only be replaced by #{partner}, who isn't on the bench."
                    ))
                }
            }
            None => {
                let on: Vec<u8> = bench
                    .into_iter()
                    .filter(|&n| self.pair_of(n).is_none())
                    .collect();
                if on.is_empty() {
                    none("Every bench player has already played in this set.".into())
                } else {
                    SubOptions { on, reason: None }
                }
            }
        }
    }
}

/// Each team's book per set, keyed by (set, team), from the visible
/// events and their fold.
pub fn books(events: &[&MatchEvent], folded: &Folded) -> HashMap<(u8, Team), SubBook> {
    let mut books: HashMap<(u8, Team), SubBook> = HashMap::new();
    for (i, e) in events.iter().enumerate() {
        let Some(Code::Substitution {
            team, out, r#in, ..
        }) = e.code()
        else {
            continue;
        };
        let (Some(off), Some(on)) = (number(out), number(r#in)) else {
            continue;
        };
        let Some(set) = folded.states.get(i).map(|s| s.set) else {
            continue;
        };
        books.entry((set, *team)).or_default().record(off, on);
    }
    books
}

fn number(d: &vb_grammar::code::Digits) -> Option<u8> {
    d.value().and_then(|v| u8::try_from(v).ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_pair_goes_out_and_back_once() {
        let mut b = SubBook::default();
        assert_eq!(b.record(3, 8), None);
        assert_eq!(
            b.pairs,
            [SubPair {
                starter: 3,
                substitute: 8,
                state: PairState::Out
            }]
        );
        assert_eq!(b.record(8, 3), None);
        assert_eq!(b.pairs[0].state, PairState::Back);
        assert_eq!(b.record(3, 8), Some(Breach::ReEntry(3)));
        assert_eq!(b.record(3, 9), Some(Breach::ReEntry(3)));
        assert_eq!(b.count, 4);
    }

    #[test]
    fn partners_are_kept() {
        let mut b = SubBook::default();
        b.record(3, 8);
        assert_eq!(
            b.record(8, 9),
            Some(Breach::OffForOther { off: 8, partner: 3 })
        );
        assert_eq!(
            b.record(4, 3),
            Some(Breach::OnForOther { on: 3, partner: 8 })
        );
        assert_eq!(b.pairs.len(), 1);
    }

    #[test]
    fn options_follow_the_pairs() {
        let mut b = SubBook::default();
        let bench = [9, 7, 10];
        assert_eq!(b.options(2, &bench).on, [7, 9, 10]);
        b.record(3, 7);
        // 7 is on court now; 3 is on the bench.
        let bench = [3, 9, 10];
        assert_eq!(b.options(7, &bench).on, [3]);
        assert_eq!(b.options(2, &bench).on, [9, 10]);
        b.record(7, 3);
        let bench = [7, 9, 10];
        let o = b.options(3, &bench);
        assert!(o.on.is_empty());
        assert!(o.reason.is_some_and(|r| r.contains("off and back")));
        assert_eq!(b.options(2, &bench).on, [9, 10]);
        let o = b.options(2, &[7]);
        assert!(o.on.is_empty());
        assert!(o.reason.is_some());
    }
}
