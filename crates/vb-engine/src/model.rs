//! A match: the two team sheets and the event log.

use vb_grammar::Team;

use crate::log::EventLog;

/// Where a player starts a set, as in the `.dvw` player table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StartingZone {
    /// On court in zone 1–6 at the start of the set.
    Zone(u8),
    /// Came on as a substitute.
    Substitute,
    /// Did not play, or not recorded.
    None,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RosterPlayer {
    pub number: u8,
    /// Sets 1–5.
    pub starting_zones: [StartingZone; 5],
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TeamSheet {
    pub players: Vec<RosterPlayer>,
}

impl TeamSheet {
    /// Players in zones 1–6 at the start of `set` (1-based). Zones nobody
    /// starts in are `None`.
    pub fn starting_lineup(&self, set: u8) -> [Option<u8>; 6] {
        let mut court = [None; 6];
        let Some(i) = usize::from(set).checked_sub(1) else {
            return court;
        };
        for p in &self.players {
            if let Some(StartingZone::Zone(z @ 1..=6)) = p.starting_zones.get(i) {
                court[usize::from(*z) - 1] = Some(p.number);
            }
        }
        court
    }
}

/// A pair of values, one per team.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct Sides<T> {
    pub home: T,
    pub visiting: T,
}

impl<T> Sides<T> {
    pub fn get(&self, team: Team) -> &T {
        match team {
            Team::Home => &self.home,
            Team::Visiting => &self.visiting,
        }
    }

    pub fn get_mut(&mut self, team: Team) -> &mut T {
        match team {
            Team::Home => &mut self.home,
            Team::Visiting => &mut self.visiting,
        }
    }
}

/// How the program that wrote the codes orders them, where that changes
/// their meaning.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Conventions {
    /// A setter change written straight after a rotation (`*z2`, `*P21`)
    /// names the setter that rotation refers to. VolleyStation does this;
    /// in DataVolley the rotation refers to the setter before the change.
    pub setter_change_follows_rotation: bool,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Match {
    pub teams: Sides<TeamSheet>,
    pub conventions: Conventions,
    pub log: EventLog,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lineup_from_starting_zones() {
        let p = |number, zones: [StartingZone; 5]| RosterPlayer {
            number,
            starting_zones: zones,
        };
        use StartingZone::{None as N, Substitute as S, Zone as Z};
        let sheet = TeamSheet {
            players: vec![
                p(8, [Z(1), Z(6), N, N, N]),
                p(12, [Z(2), Z(1), N, N, N]),
                p(5, [S, Z(2), N, N, N]),
            ],
        };
        assert_eq!(
            sheet.starting_lineup(1),
            [Some(8), Some(12), None, None, None, None]
        );
        assert_eq!(
            sheet.starting_lineup(2),
            [Some(12), Some(5), None, None, None, Some(8)]
        );
        assert_eq!(sheet.starting_lineup(0), [None; 6]);
        assert_eq!(sheet.starting_lineup(6), [None; 6]);
    }
}
