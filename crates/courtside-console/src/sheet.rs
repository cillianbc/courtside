//! Scoresheets (issue #57): a match started from one, one attached to a
//! match, and a coded match checked against it. The sheet is read by
//! `vb_scoresheet`; here it meets the season (which team is which, which
//! players are new) and the coded log.

use serde::{Deserialize, Serialize};
use vb_engine::{Match, fold};
use vb_grammar::{Code, Team};
use vb_scoresheet::Scoresheet;

use crate::snapshot::{Pair, Side};

/// What a scoresheet says, and how it meets the season, before a match is
/// made from it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SheetPreview {
    pub sheet: Scoresheet,
    /// Per sheet team (home first).
    pub teams: [TeamMatch; 2],
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TeamMatch {
    /// The season team this looks like, if any.
    pub team_id: Option<String>,
    pub team_name: Option<String>,
    /// Shirt numbers on the sheet that the team's roster doesn't have.
    pub new_players: Vec<u8>,
    /// Numbers on both whose last names differ: `#7 roster Smith, sheet Jones`.
    pub differing: Vec<String>,
}

/// The scout's choices on the review screen.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct SheetChoice {
    /// Per sheet team: the season team to use; `None` adds the sheet's.
    pub team_ids: [Option<String>; 2],
}

/// The sheet's sixes, first servers and scores for the lineup screen, by
/// side.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SheetSummary {
    pub source: String,
    pub sets: Vec<SheetSetView>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SheetSetView {
    pub set: u8,
    /// Zones 1–6 (the sheet's I–VI); empty when the sheet has none.
    pub sixes: Pair<Vec<u8>>,
    pub serving: Option<Side>,
    pub score: Option<Pair<u8>>,
}

/// Where the coded match and the sheet disagree, set by set.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SheetCheck {
    /// Sets compared (those with coded rallies).
    pub sets: Vec<u8>,
    pub differences: Vec<String>,
}

fn same(a: &str, b: &str) -> bool {
    a.trim().eq_ignore_ascii_case(b.trim())
}

/// Which season team a sheet team is: the same name, else one name inside
/// the other ("Harbour City" for "Harbour City VC").
pub fn find_team<'a>(
    teams: &'a [courtside_store::Team],
    name: &str,
) -> Option<&'a courtside_store::Team> {
    let n = name.trim().to_lowercase();
    teams.iter().find(|t| same(&t.name, name)).or_else(|| {
        teams.iter().find(|t| {
            let m = t.name.trim().to_lowercase();
            !m.is_empty() && !n.is_empty() && (n.contains(&m) || m.contains(&n))
        })
    })
}

/// The sheet's teams against the season's.
pub fn preview(sheet: Scoresheet, season: &[courtside_store::Team]) -> SheetPreview {
    let teams = [0, 1].map(|i| {
        let st = &sheet.teams[i];
        let Some(t) = find_team(season, &st.name) else {
            return TeamMatch {
                new_players: st.players.iter().map(|p| p.number).collect(),
                ..TeamMatch::default()
            };
        };
        let mut m = TeamMatch {
            team_id: Some(t.id.clone()),
            team_name: Some(t.name.clone()),
            ..TeamMatch::default()
        };
        for p in &st.players {
            match t.players.iter().find(|r| r.number == p.number) {
                None => m.new_players.push(p.number),
                Some(r) if !same(&r.last_name, &p.last_name) => m.differing.push(format!(
                    "#{} roster {}, sheet {}",
                    p.number, r.last_name, p.last_name
                )),
                Some(_) => {}
            }
        }
        m
    });
    SheetPreview { sheet, teams }
}

/// Which side each sheet team plays on in a match: by name, else home
/// first as the sheet's title has it.
pub fn sides(sheet: &Scoresheet, home: &str, visiting: &str) -> [Team; 2] {
    let [a, b] = [&sheet.teams[0].name, &sheet.teams[1].name];
    if (same(a, visiting) || same(b, home)) && !(same(a, home) || same(b, visiting)) {
        [Team::Visiting, Team::Home]
    } else {
        [Team::Home, Team::Visiting]
    }
}

fn by_side<T: Clone>(sides: [Team; 2], v: &[T; 2]) -> Pair<T> {
    let i = usize::from(sides[0] != Team::Home);
    Pair {
        home: v[i].clone(),
        visiting: v[1 - i].clone(),
    }
}

pub fn summary(sheet: &Scoresheet, home: &str, visiting: &str) -> SheetSummary {
    let sides = sides(sheet, home, visiting);
    SheetSummary {
        source: sheet.source.clone(),
        sets: sheet
            .sets
            .iter()
            .map(|s| SheetSetView {
                set: s.number,
                sixes: by_side(sides, &s.lineups),
                serving: s.served_first.map(|t| Side::from(sides[t])),
                score: s.score.map(|sc| by_side(sides, &sc)),
            })
            .collect(),
    }
}

/// A substitution, own score first.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Sub {
    team: Team,
    out: u8,
    on: u8,
    score: Option<[u8; 2]>,
}

fn own(team: Team, home: u8, visiting: u8) -> [u8; 2] {
    match team {
        Team::Home => [home, visiting],
        Team::Visiting => [visiting, home],
    }
}

fn six(v: &[u8]) -> String {
    v.iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join(" ")
}

fn score(s: Option<[u8; 2]>) -> String {
    s.map_or("?".into(), |[a, b]| format!("{a}:{b}"))
}

/// The coded match against the sheet, for the sets with coded rallies.
pub fn check(game: &Match, sheet: &Scoresheet, names: Pair<&str>) -> SheetCheck {
    let sides = sides(sheet, names.home, names.visiting);
    let name = |t: Team| match t {
        Team::Home => names.home,
        Team::Visiting => names.visiting,
    };
    let folded = fold(game);
    let events = game.log.visible();
    let mut out = SheetCheck::default();
    for s in &sheet.sets {
        let rallies: Vec<_> = folded
            .rallies
            .iter()
            .filter(|r| r.set == s.number)
            .collect();
        let Some(last) = rallies.last() else { continue };
        out.sets.push(s.number);
        let mut say = |m: String| out.differences.push(format!("Set {}: {m}", s.number));

        // The sixes, once both are on court in this set.
        let full = |st: &&vb_engine::fold::State| {
            st.set == s.number
                && [Team::Home, Team::Visiting]
                    .iter()
                    .all(|t| st.court.get(*t).iter().all(Option::is_some))
        };
        if let Some(first) = folded.states.iter().find(full) {
            for (i, sheet_six) in s.lineups.iter().enumerate() {
                let team = sides[i];
                let coded: Vec<u8> = first
                    .court
                    .get(team)
                    .iter()
                    .map(|z| z.unwrap_or(0))
                    .collect();
                if !sheet_six.is_empty() && *sheet_six != coded {
                    say(format!(
                        "{}'s six is {} on the sheet, {} coded",
                        name(team),
                        six(sheet_six),
                        six(&coded)
                    ));
                }
            }
        }

        if let Some([a, b]) = s.score {
            let sheet_score = Pair {
                home: if sides[0] == Team::Home { a } else { b },
                visiting: if sides[0] == Team::Home { b } else { a },
            };
            let coded = last.score_after;
            if (coded.home, coded.visiting) != (sheet_score.home, sheet_score.visiting) {
                say(format!(
                    "the sheet says {}:{}, coded {}:{} ({} first)",
                    sheet_score.home, sheet_score.visiting, coded.home, coded.visiting, names.home
                ));
            }
        }

        // Substitutions: each on the sheet (and the starter's return) is a
        // `c` code; scores are the team's own first, as the sheet writes them.
        let mut expected: Vec<Sub> = Vec::new();
        for x in &s.substitutions {
            let team = sides[x.team];
            expected.push(Sub {
                team,
                out: x.player_out,
                on: x.player_in,
                score: x.score_in,
            });
            if x.score_back.is_some() {
                expected.push(Sub {
                    team,
                    out: x.player_in,
                    on: x.player_out,
                    score: x.score_back,
                });
            }
        }
        let mut coded: Vec<Sub> = Vec::new();
        for (i, e) in events.iter().enumerate() {
            let Some(st) = folded.states.get(i) else {
                continue;
            };
            if st.set != s.number {
                continue;
            }
            if let Some(Code::Substitution {
                team, out, r#in, ..
            }) = e.code()
            {
                let num = |d: &vb_grammar::code::Digits| {
                    d.value().and_then(|v| u8::try_from(v).ok()).unwrap_or(0)
                };
                coded.push(Sub {
                    team: *team,
                    out: num(out),
                    on: num(r#in),
                    score: Some(own(*team, st.score.home, st.score.visiting)),
                });
            }
        }
        for x in &expected {
            match coded
                .iter()
                .position(|c| c.team == x.team && c.out == x.out && c.on == x.on)
            {
                Some(k) => {
                    let c = coded.remove(k);
                    if x.score.is_some() && c.score != x.score {
                        say(format!(
                            "{} {} → {} at {} on the sheet, coded at {}",
                            name(x.team),
                            x.out,
                            x.on,
                            score(x.score),
                            score(c.score)
                        ));
                    }
                }
                None => say(format!(
                    "{} {} → {} at {} on the sheet isn't coded",
                    name(x.team),
                    x.out,
                    x.on,
                    score(x.score)
                )),
            }
        }
        for c in coded {
            say(format!(
                "{} {} → {} at {} is coded but not on the sheet",
                name(c.team),
                c.out,
                c.on,
                score(c.score)
            ));
        }
    }
    out
}
