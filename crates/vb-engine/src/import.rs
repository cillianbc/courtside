//! `.dvw` to [`Match`]: one event per `[3SCOUT]` line, in file order.

use dvw_codec::Dvw;
use dvw_codec::dvw::Player;
use vb_grammar::Code;

use crate::event::{ClockTime, IdGen, ImportFormat, NewEvent, Producer};
use crate::model::{Conventions, Match, RosterPlayer, Sides, StartingZone, TeamSheet};

impl Match {
    /// Every `[3SCOUT]` line becomes one event (an empty line becomes an
    /// unrecognised empty code), so event `seq` equals the line index and
    /// `source_line` points back at it. Players without a readable shirt
    /// number are left off the team sheet.
    pub fn from_dvw(dvw: &Dvw, ids: &mut IdGen) -> Self {
        let mut m = Match {
            teams: Sides {
                home: sheet(&dvw.players_home),
                visiting: sheet(&dvw.players_visiting),
            },
            conventions: Conventions {
                setter_change_follows_rotation: dvw.header.get("GENERATOR-PRG")
                    == Some("VolleyStation"),
            },
            ..Match::default()
        };
        for (i, line) in dvw.scout.iter().enumerate() {
            let new = NewEvent {
                video_ms: line
                    .video_time()
                    .filter(|s| s.is_finite() && *s >= 0.0)
                    .map(|s| (s * 1000.0).round() as u64),
                clock: line.time_hms().map(|(hour, minute, second)| ClockTime {
                    hour,
                    minute,
                    second,
                }),
                ..NewEvent::code(
                    Code::parse(line.code()),
                    Producer::Import(ImportFormat::Dvw),
                )
            };
            m.log.push(ids, new, None, Some(i));
        }
        m
    }
}

fn sheet(players: &[Player]) -> TeamSheet {
    TeamSheet {
        players: players
            .iter()
            .filter_map(|p| {
                Some(RosterPlayer {
                    number: p.number()?,
                    starting_zones: std::array::from_fn(|i| {
                        match p.starting_zone(i + 1).map(str::trim) {
                            Some("*") => StartingZone::Substitute,
                            Some(z) => z
                                .parse::<u8>()
                                .ok()
                                .filter(|z| (1..=6).contains(z))
                                .map_or(StartingZone::None, StartingZone::Zone),
                            None => StartingZone::None,
                        }
                    }),
                })
            })
            .collect(),
    }
}
