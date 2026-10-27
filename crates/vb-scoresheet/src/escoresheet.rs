//! eScoresheet's printed scoresheet (the FIVB layout, one A4 page in
//! landscape). Fields are found from the form's fixed text, not their
//! order: the Roman numerals over each team's lineup, the large set
//! numbers, the results table's headings, "Name of the player", "LIBERO",
//! "APPROVAL". Measurements are from the samples' 842 × 595 page (local,
//! issue #57); row labels differ between releases ("Rotation" / "Service
//! Order"), so they aren't used.

use crate::pdf::{Circle, Page, Run};
use crate::{Error, Scoresheet, SheetPlayer, SheetSet, SheetStaff, SheetTeam, Substitution};

/// A row of the results table.
struct SetResult {
    set: u8,
    /// By team index.
    points: [u8; 2],
    minutes: Option<u16>,
}

/// Runs on one baseline differ by less than this (points).
const SAME_LINE: f64 = 1.0;

fn trimmed(r: &Run) -> &str {
    r.text.trim()
}

/// Where a run's text is centred, roughly: Arial digits are about half the
/// font size wide.
fn centre(r: &Run) -> f64 {
    r.x + 0.27 * r.size * trimmed(r).chars().count() as f64
}

fn int(r: &Run) -> Option<u8> {
    let t = trimmed(r);
    (!t.is_empty() && t.len() <= 3 && t.chars().all(|c| c.is_ascii_digit()))
        .then(|| t.parse().ok())
        .flatten()
}

/// `15:11` as two numbers.
fn pair(s: &str) -> Option<[u8; 2]> {
    let (a, b) = s.trim().split_once(':')?;
    Some([a.trim().parse().ok()?, b.trim().parse().ok()?])
}

const ROMAN: [&str; 6] = ["I", "II", "III", "IV", "V", "VI"];

/// `Last (N0000) First` → last, first, id. Names without an id are
/// "Last First" (the first word is the last name).
fn name_parts(s: &str) -> (String, String, String) {
    let s = s.trim();
    if let (Some(open), Some(close)) = (s.find('('), s.find(')'))
        && open < close
    {
        return (
            s[..open].trim().to_owned(),
            s[close + 1..].trim().to_owned(),
            s[open + 1..close].trim().to_owned(),
        );
    }
    let (last, first) = s.split_once(' ').unwrap_or((s, ""));
    (
        last.trim().to_owned(),
        first.trim().to_owned(),
        String::new(),
    )
}

/// `25 Oct 2026` → `2026-10-25`.
fn iso_date(s: &str) -> Option<String> {
    const MONTHS: [&str; 12] = [
        "jan", "feb", "mar", "apr", "may", "jun", "jul", "aug", "sep", "oct", "nov", "dec",
    ];
    let mut it = s.split_whitespace();
    let day: u8 = it.next()?.parse().ok()?;
    let month = it.next()?.to_ascii_lowercase();
    let m = MONTHS.iter().position(|x| month.starts_with(x))? + 1;
    let year: u16 = it.next()?.parse().ok()?;
    ((1..=31).contains(&day) && year > 1900).then(|| format!("{year:04}-{m:02}-{day:02}"))
}

/// `17.30.00` or `17.30` → `17:30`.
fn clock(s: &str) -> String {
    let t = s.trim();
    let mut parts = t.split('.');
    match (parts.next(), parts.next()) {
        (Some(h), Some(m)) if h.len() <= 2 && m.len() == 2 => format!("{h:0>2}:{m}"),
        _ => t.to_owned(),
    }
}

struct Sheet<'a> {
    runs: &'a [Run],
    circles: &'a [Circle],
}

impl<'a> Sheet<'a> {
    fn find(&self, text: &str) -> impl Iterator<Item = &'a Run> + use<'a, '_> {
        let text = text.to_owned();
        self.runs.iter().filter(move |r| trimmed(r) == text)
    }

    /// The nearest run on `label`'s line to its right, within `max_dx`.
    fn right_of(&self, label: &Run, max_dx: f64, dy: f64) -> Option<&'a Run> {
        self.runs
            .iter()
            .filter(|r| {
                r.x > label.x + 1.0
                    && r.x < label.x + max_dx
                    && (r.y - label.y).abs() < dy
                    && !trimmed(r).is_empty()
            })
            .min_by(|a, b| a.x.total_cmp(&b.x))
    }

    fn value(&self, label: &str) -> String {
        self.find(label)
            .next()
            .and_then(|l| self.right_of(l, 150.0, SAME_LINE))
            .map(|r| trimmed(r).to_owned())
            .unwrap_or_default()
    }

    fn circled(&self, r: &Run) -> bool {
        let (x, y) = (centre(r), r.y - r.size * 0.35);
        self.circles.iter().any(|c| c.contains(x, y))
    }

    /// Which team a name is (0 left / home, 1 right), by the title names.
    fn team_of(names: &[String; 2], text: &str) -> Option<usize> {
        let t = text.trim().to_lowercase();
        names
            .iter()
            .position(|n| !n.is_empty() && n.to_lowercase() == t)
    }
}

pub fn parse(page: &Page) -> Result<Scoresheet, Error> {
    let s = Sheet {
        runs: &page.runs,
        circles: &page.circles,
    };
    let source = s
        .runs
        .iter()
        .find(|r| r.text.contains("eScoresheet"))
        .map(|r| {
            trimmed(r)
                .trim_start_matches("Printed by")
                .replace("Release ", "")
                .trim()
                .to_owned()
        })
        .ok_or_else(|| Error::NotScoresheet("no eScoresheet footer".into()))?;

    // The title: the two team names in large bold type, home on the left.
    let mut titles: Vec<&Run> = s
        .runs
        .iter()
        .filter(|r| r.bold && r.size >= 11.5 && r.y < 60.0 && !trimmed(r).is_empty())
        .collect();
    titles.sort_by(|a, b| a.x.total_cmp(&b.x));
    let [left, right] = titles.as_slice() else {
        return Err(Error::NotScoresheet("no team names in the title".into()));
    };
    let names = [trimmed(left).to_owned(), trimmed(right).to_owned()];
    // The A/B marks between them: the first follows the left name.
    let mut marks: Vec<&Run> = s
        .runs
        .iter()
        .filter(|r| {
            matches!(trimmed(r), "A" | "B")
                && (r.y - left.y).abs() < 8.0
                && r.x > left.x
                && r.x < right.x + 5.0
        })
        .collect();
    marks.sort_by(|a, b| a.x.total_cmp(&b.x));
    let letters = match marks.as_slice() {
        [a, b, ..] => [trimmed(a).to_owned(), trimmed(b).to_owned()],
        _ => [String::new(), String::new()],
    };

    let header_y = left.y;
    let competition = s
        .runs
        .iter()
        .filter(|r| r.bold && r.size >= 9.0 && r.size < 11.5 && r.y < header_y && r.x < 150.0)
        .min_by(|a, b| a.y.total_cmp(&b.y))
        .map(|r| trimmed(r).to_owned())
        .unwrap_or_default();
    let category = s
        .runs
        .iter()
        .find(|r| r.italic && r.y < header_y)
        .map(|r| trimmed(r).to_owned())
        .unwrap_or_default();
    let date = s
        .runs
        .iter()
        .filter(|r| r.y < header_y)
        .find_map(|r| iso_date(trimmed(r)));
    let time = s
        .runs
        .iter()
        .filter(|r| r.y < header_y)
        .map(trimmed)
        .find(|t| {
            let p: Vec<&str> = t.split('.').collect();
            (2..=3).contains(&p.len())
                && p.iter()
                    .all(|x| x.len() == 2 && x.chars().all(|c| c.is_ascii_digit()))
        })
        .map(clock)
        .unwrap_or_default();

    let mut teams: [SheetTeam; 2] = [0, 1].map(|i| SheetTeam {
        name: names[i].clone(),
        letter: letters[i].clone(),
        ..SheetTeam::default()
    });
    rosters(&s, &names, &mut teams);
    let sets = sets(&s, &names, &letters);
    let (referees, scorer, assistant_scorer) = officials(&s);

    Ok(Scoresheet {
        source,
        competition,
        category,
        match_number: s.value("Match no."),
        day: s.value("Day:"),
        date,
        time,
        city: s.value("City:"),
        hall: s.value("Hall:"),
        country: s.value("Country:"),
        teams,
        sets,
        referees,
        scorer,
        assistant_scorer,
    })
}

/// The team lists, liberos and bench officials (bottom right).
fn rosters(s: &Sheet<'_>, names: &[String; 2], teams: &mut [SheetTeam; 2]) {
    let headers: Vec<&Run> = s.find("Name of the player").collect();
    let Some(top) = headers.iter().map(|h| h.y).reduce(f64::min) else {
        return;
    };
    // Each column's team: the bold name just above its header.
    let column_team = |h: &Run| -> Option<usize> {
        s.runs
            .iter()
            .filter(|r| r.bold && r.y < h.y && r.y > h.y - 25.0 && (r.x - h.x).abs() < 20.0)
            .find_map(|r| Sheet::team_of(names, &r.text))
    };
    let columns: Vec<(f64, usize)> = headers
        .iter()
        .filter_map(|h| column_team(h).map(|t| (h.x, t)))
        .collect();
    let team_at = |x: f64| -> Option<usize> {
        columns
            .iter()
            .min_by(|a, b| (a.0 - x).abs().total_cmp(&(b.0 - x).abs()))
            .filter(|(cx, _)| (cx - x).abs() < 30.0)
            .map(|(_, t)| *t)
    };
    let libero_y = s.find("LIBERO").next().map_or(f64::INFINITY, |r| r.y);
    let staff_y = s
        .runs
        .iter()
        .find(|r| matches!(trimmed(r), "OFFICIALS") || r.text.contains("ADMITTED ON THE BENCH"))
        .map_or(f64::INFINITY, |r| r.y);
    let end_y = s.find("SIGNATURES").next().map_or(f64::INFINITY, |r| r.y);

    // A bold number, then the name on its line.
    for num in s.runs.iter().filter(|r| r.y > top && r.y < libero_y + 30.0) {
        let Some(number) = int(num) else { continue };
        if !num.bold || number > 99 {
            continue;
        }
        let Some(name) = s
            .runs
            .iter()
            .filter(|r| {
                r.x > num.x
                    && r.x < num.x + 20.0
                    && (r.y - num.y).abs() < SAME_LINE
                    && trimmed(r).contains('(')
            })
            .min_by(|a, b| a.x.total_cmp(&b.x))
        else {
            continue;
        };
        let Some(t) = team_at(name.x) else { continue };
        let libero = num.y > libero_y;
        let (last_name, first_name, id) = name_parts(&name.text);
        let player = SheetPlayer {
            number,
            last_name,
            first_name,
            id,
            libero,
            captain: s.circled(num),
        };
        let list = &mut teams[t].players;
        match list.iter_mut().find(|p| p.number == number) {
            // A libero listed in the team list as well.
            Some(p) => {
                p.libero |= player.libero;
                p.captain |= player.captain;
            }
            None => list.push(player),
        }
    }
    for t in teams.iter_mut() {
        t.players.sort_by_key(|p| p.number);
    }

    // Bench officials: a role (`C`, `AC`, …), then the name.
    for role in s.runs.iter().filter(|r| r.y > staff_y && r.y < end_y) {
        let code = trimmed(role);
        if code.is_empty() || code.len() > 3 || !code.chars().all(|c| c.is_ascii_uppercase()) {
            continue;
        }
        let Some(name) = s.right_of(role, 25.0, SAME_LINE) else {
            continue;
        };
        let Some(t) = team_at(name.x) else { continue };
        let (last_name, first_name, id) = name_parts(&name.text);
        teams[t].staff.push(SheetStaff {
            role: code.to_owned(),
            last_name,
            first_name,
            id,
        });
    }
}

/// Referees, scorer and assistant scorer from the approval block.
fn officials(s: &Sheet<'_>) -> (Vec<String>, String, String) {
    let Some(approval) = s.find("APPROVAL").next() else {
        return (Vec::new(), String::new(), String::new());
    };
    let name = |label: &str| -> String {
        s.runs
            .iter()
            .find(|r| trimmed(r) == label && r.y > approval.y && (r.x - approval.x).abs() < 200.0)
            .and_then(|l| s.right_of(l, 150.0, 4.0).filter(|r| r.bold))
            .map(|r| trimmed(r).to_owned())
            .unwrap_or_default()
    };
    let referees = ["1°", "2°"]
        .map(name)
        .into_iter()
        .filter(|n| !n.is_empty())
        .collect();
    (referees, name("Scorer"), name("Assistant"))
}

/// One team's half of a set's box.
struct Half {
    team: usize,
    after_change: bool,
    lineup: Vec<u8>,
    subs: Vec<Substitution>,
    serves: Option<bool>,
    start: String,
    end: String,
}

fn half(s: &Sheet<'_>, name: &Run, team: usize, after_change: bool) -> Option<Half> {
    let hy = name.y;
    // The Roman numerals over the six columns.
    let mut cols: Vec<(u8, f64)> = Vec::new();
    for (i, numeral) in ROMAN.iter().enumerate() {
        let r = s.runs.iter().find(|r| {
            trimmed(r) == *numeral
                && r.y > hy + 6.0
                && r.y < hy + 16.0
                && r.x > name.x - 15.0
                && r.x < name.x + 120.0
        })?;
        cols.push((u8::try_from(i + 1).ok()?, centre(r)));
    }
    let column = |r: &Run| -> Option<u8> {
        cols.iter()
            .min_by(|a, b| (a.1 - centre(r)).abs().total_cmp(&(b.1 - centre(r)).abs()))
            .filter(|(_, x)| (x - centre(r)).abs() < 10.0)
            .map(|(p, _)| *p)
    };
    let in_band = |r: &Run, from: f64, to: f64| {
        r.y > hy + from && r.y < hy + to && r.x > name.x - 15.0 && r.x < name.x + 125.0
    };
    let mut lineup = [0u8; 6];
    for r in s.runs.iter().filter(|r| in_band(r, 16.0, 24.0)) {
        if let (Some(n), Some(p)) = (int(r), column(r)) {
            lineup[usize::from(p - 1)] = n;
        }
    }
    let mut subs: Vec<Substitution> = Vec::new();
    for r in s.runs.iter().filter(|r| in_band(r, 28.0, 40.0)) {
        if let (Some(n), Some(p)) = (int(r), column(r)) {
            subs.push(Substitution {
                team,
                position: p,
                player_out: lineup[usize::from(p - 1)],
                player_in: n,
                ..Substitution::default()
            });
        }
    }
    let mut scores: Vec<&Run> = s
        .runs
        .iter()
        .filter(|r| in_band(r, 40.0, 63.0) && pair(&r.text).is_some())
        .collect();
    scores.sort_by(|a, b| a.y.total_cmp(&b.y));
    for r in scores {
        let Some(p) = column(r) else { continue };
        if let Some(sub) = subs.iter_mut().find(|x| x.position == p) {
            if sub.score_in.is_none() {
                sub.score_in = pair(&r.text);
            } else if sub.score_back.is_none() {
                sub.score_back = pair(&r.text);
            }
        }
    }
    let header: Vec<&Run> = s
        .runs
        .iter()
        .filter(|r| (r.y - hy).abs() < 1.5 && r.x > name.x && r.x < name.x + 160.0)
        .collect();
    let serves = header.iter().find_map(|r| match trimmed(r) {
        "S" => Some(true),
        "R" => Some(false),
        _ => None,
    });
    let time = |prefix: &str| {
        header
            .iter()
            .find_map(|r| trimmed(r).strip_prefix(prefix).map(clock))
            .unwrap_or_default()
    };
    Some(Half {
        team,
        after_change,
        lineup: if lineup.contains(&0) {
            Vec::new()
        } else {
            lineup.to_vec()
        },
        subs,
        serves,
        start: time("Start:"),
        end: time("End:"),
    })
}

fn sets(s: &Sheet<'_>, names: &[String; 2], letters: &[String; 2]) -> Vec<SheetSet> {
    let results = results(s, letters);
    let mut out = Vec::new();
    // The set numbers: large bold digits in each box's "SET" label.
    let numbers: Vec<(&Run, u8)> = s
        .runs
        .iter()
        .filter(|r| r.bold && r.size >= 10.0 && r.y > 50.0)
        .filter_map(|r| int(r).filter(|n| (1..=5).contains(n)).map(|n| (r, n)))
        .collect();
    for &(num, n) in &numbers {
        // A box ends where the next one in its row starts (set 5's, with
        // its third half after the change of ends, runs on).
        let end = numbers
            .iter()
            .filter(|(o, _)| (o.y - num.y).abs() < 3.0 && o.x > num.x + 1.0)
            .map(|(o, _)| o.x)
            .reduce(f64::min)
            .unwrap_or(num.x + 560.0);
        if out.iter().any(|x: &SheetSet| x.number == n) {
            continue;
        }
        // The box's halves: a team name across its top.
        let mut halves: Vec<Half> = s
            .runs
            .iter()
            .filter(|r| {
                r.bold && r.y > num.y - 72.0 && r.y < num.y - 55.0 && r.x > num.x && r.x < end
            })
            .filter_map(|r| {
                let team = Sheet::team_of(names, &r.text)?;
                let after_change = s.runs.iter().any(|c| {
                    c.text.contains("Points at Change")
                        && (c.y - r.y).abs() < 1.5
                        && c.x > r.x
                        && c.x < r.x + 160.0
                });
                half(s, r, team, after_change)
            })
            .collect();
        halves.sort_by_key(|h| h.after_change);
        let mut set = SheetSet {
            number: n,
            ..SheetSet::default()
        };
        for h in halves {
            if !h.after_change && set.lineups[h.team].is_empty() {
                set.lineups[h.team].clone_from(&h.lineup);
                if h.serves == Some(true) {
                    set.served_first = Some(h.team);
                } else if h.serves == Some(false) && set.served_first.is_none() {
                    set.served_first = Some(1 - h.team);
                }
                if !h.start.is_empty() {
                    set.start.clone_from(&h.start);
                }
                if !h.end.is_empty() {
                    set.end.clone_from(&h.end);
                }
            }
            for sub in h.subs {
                // After the change of ends the lineup is copied over; a
                // substitution there is new only if not already recorded.
                if !set.substitutions.contains(&sub) {
                    set.substitutions.push(sub);
                }
            }
        }
        if set.lineups.iter().all(Vec::is_empty) {
            continue;
        }
        if let Some(r) = results.iter().find(|r| r.set == n) {
            set.score = Some(r.points);
            set.minutes = r.minutes;
        }
        out.push(set);
    }
    out.sort_by_key(|x| x.number);
    out
}

/// The results table: per set, each team's points and the set's minutes,
/// by team index (`letters` says which of A and B is which team).
fn results(s: &Sheet<'_>, letters: &[String; 2]) -> Vec<SetResult> {
    let Some(head) = s
        .runs
        .iter()
        .find(|r| trimmed(r).starts_with("Set") && r.text.contains("Time"))
    else {
        return Vec::new();
    };
    // The two P headings: team A's left of Set, team B's right.
    let ps: Vec<&Run> = s.find("P").filter(|r| (r.y - head.y).abs() < 1.5).collect();
    let (Some(pa), Some(pb)) = (
        ps.iter()
            .filter(|r| r.x < head.x)
            .max_by(|a, b| a.x.total_cmp(&b.x)),
        ps.iter()
            .filter(|r| r.x > head.x)
            .min_by(|a, b| a.x.total_cmp(&b.x)),
    ) else {
        return Vec::new();
    };
    let a_is = letters.iter().position(|l| l == "A").unwrap_or(0);
    let mut rows: Vec<&Run> = s
        .runs
        .iter()
        .filter(|r| r.y > head.y + 3.0 && r.y < head.y + 60.0 && (r.x - head.x).abs() < 6.0)
        .collect();
    rows.sort_by(|a, b| a.y.total_cmp(&b.y));
    rows.iter()
        .filter_map(|set| {
            let n = int(set).filter(|n| (1..=5).contains(n))?;
            let on_row = |x: f64| {
                s.runs
                    .iter()
                    .filter(|r| (r.y - set.y).abs() < SAME_LINE && (r.x - x).abs() < 6.0)
                    .find_map(int)
            };
            let (a, b) = (on_row(pa.x)?, on_row(pb.x)?);
            let minutes = s
                .runs
                .iter()
                .filter(|r| (r.y - set.y).abs() < SAME_LINE && r.x > set.x && r.x < pb.x)
                .find_map(|r| trimmed(r).trim_start_matches(['\'', '’']).parse().ok());
            let points = if a_is == 0 { [a, b] } else { [b, a] };
            Some(SetResult {
                set: n,
                points,
                minutes,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A made-up sheet in eScoresheet's geometry: two sets, one
    /// substitution (with the starter back), rosters, a libero, a
    /// captain's box, a coach, two referees and the results.
    fn page() -> Page {
        let mut runs = Vec::new();
        let mut t = |x: f64, y: f64, size: f64, bold: bool, text: &str| {
            runs.push(Run {
                x,
                y,
                size,
                bold,
                italic: false,
                text: text.into(),
            });
        };
        t(2.8, 14.9, 10.0, true, "Harbour League - Men");
        t(275.0, 10.0, 8.0, false, "Match no.");
        t(314.6, 10.2, 8.0, true, "42");
        t(365.7, 10.0, 8.0, false, "Day:");
        t(394.0, 10.2, 8.0, true, "7");
        t(431.1, 10.2, 8.0, true, "14 Mar 2027");
        t(437.7, 24.4, 8.0, true, "19.45.00");
        t(510.2, 10.0, 8.0, false, "City:");
        t(549.9, 10.2, 8.0, true, "Harbourtown");
        t(510.2, 24.1, 8.0, false, "Hall:");
        t(549.9, 24.4, 8.0, true, "Quayside Arena");
        t(293.1, 42.3, 12.0, true, "Harbour City");
        t(398.8, 37.4, 7.0, false, "B");
        t(435.7, 37.4, 7.0, false, "A");
        t(447.9, 42.3, 12.0, true, "Northgate");
        // Sets 1 and 2 side by side; in each, the left half is the first
        // team named.
        for (set, x0, left, right, serve) in
            [(1u8, 79.4, 1usize, 0usize, "R"), (2, 425.2, 0, 1, "S")]
        {
            let names = ["Harbour City", "Northgate"];
            t(x0, 125.0, 11.0, true, &format!(" {set}"));
            for (h, team) in [(0.0, left), (164.4, right)] {
                let nx = x0 + 25.5 + h;
                t(
                    nx - 9.4,
                    60.1,
                    7.0,
                    false,
                    if team == 0 { "B" } else { "A" },
                );
                t(nx, 60.3, 7.0, true, names[team]);
                t(
                    nx + 138.9,
                    60.1,
                    7.0,
                    false,
                    if h == 0.0 {
                        serve
                    } else if serve == "S" {
                        "R"
                    } else {
                        "S"
                    },
                );
                if h == 0.0 {
                    t(nx + 93.5, 60.1, 7.0, false, "Start: 19.45");
                }
                for (i, numeral) in ROMAN.iter().enumerate() {
                    t(nx - 5.1 + 19.5 * i as f64, 71.4, 7.0, false, numeral);
                }
                let six: [u8; 6] = if team == 0 {
                    [1, 2, 3, 4, 5, 6]
                } else {
                    [11, 12, 13, 14, 15, 16]
                };
                for (i, n) in six.iter().enumerate() {
                    t(nx - 6.3 + 19.5 * i as f64, 79.9, 7.0, false, &n.to_string());
                }
            }
        }
        // Set 1: Harbour City (right half) brings 7 on for 3 at 8:5; 3 back at 20:18.
        // Column III of set 1's right half (nx 269.3): the number where the
        // starter's is, the scores a little left as they're wider.
        let col3 = 79.4 + 25.5 + 164.4 - 6.3 + 19.5 * 2.0;
        t(col3, 94.1, 7.0, false, "7");
        t(col3 - 4.8, 105.5, 7.0, false, "8:5");
        t(col3 - 4.8, 117.0, 7.0, false, "20:18");
        // Rosters.
        t(588.8, 272.7, 7.0, false, "B");
        t(598.1, 272.9, 7.0, true, "Harbour City");
        t(714.3, 272.9, 7.0, true, "Northgate");
        t(598.1, 287.1, 7.0, true, "Name of the player");
        t(725.7, 287.1, 7.0, true, "Name of the player");
        for (i, n) in (1..=7).enumerate() {
            let y = 295.6 + 8.5 * i as f64;
            t(587.6, y, 7.0, true, &n.to_string());
            t(
                598.1,
                y - 0.2,
                7.0,
                false,
                &format!("PLAYER{n} (N00{n}) Home"),
            );
            t(713.2, y, 7.0, true, &(n + 10).to_string());
            t(
                725.7,
                y - 0.2,
                7.0,
                false,
                &format!("VISITOR{n} (N10{n}) Away"),
            );
        }
        t(698.5, 411.8, 7.0, true, "LIBERO");
        t(587.3, 423.2, 7.0, true, "L");
        t(599.0, 423.2, 7.0, true, "9");
        t(609.4, 422.9, 7.0, false, "KEEPER (N0099) Lib");
        t(693.4, 457.2, 7.0, true, "OFFICIALS");
        t(587.1, 468.3, 7.0, false, "C");
        t(598.1, 468.5, 7.0, true, "Marlow (C0001) Dennis");
        t(688.8, 479.8, 7.0, true, "SIGNATURES");
        // Approval.
        t(241.8, 448.7, 7.0, true, "APPROVAL");
        t(125.3, 469.3, 5.0, false, "1°");
        t(130.0, 467.0, 7.0, true, "Okafor Tess");
        t(120.2, 492.0, 5.0, false, "Scorer");
        t(130.0, 489.7, 7.0, true, "Vance Ida");
        // Results: A (Northgate) left, B (Harbour City) right.
        t(473.4, 469.3, 5.0, false, "Set    Time");
        t(458.9, 469.3, 5.0, false, "P");
        t(515.6, 469.3, 5.0, false, "P");
        for (i, (set, a, b, min)) in [(1, "25", "21", "'24"), (2, "19", "25", "'22")]
            .iter()
            .enumerate()
        {
            let y = 479.6 + 8.5 * i as f64;
            t(456.7, y, 7.0, false, a);
            t(474.2, y, 7.0, false, &set.to_string());
            t(492.9, y, 7.0, false, min);
            t(513.4, y, 7.0, false, b);
        }
        t(
            0.0,
            593.2,
            7.0,
            true,
            "Printed by eScoresheet Release 2027.03.01",
        );
        Page {
            width: 842.0,
            height: 595.0,
            runs,
            // Harbour City's captain, #2.
            circles: vec![Circle {
                x0: 584.0,
                y0: 298.0,
                x1: 595.0,
                y1: 306.0,
            }],
        }
    }

    #[test]
    fn reads_a_made_up_sheet() {
        let s = parse(&page()).expect("sheet");
        assert_eq!(s.source, "eScoresheet 2027.03.01");
        assert_eq!(
            (
                s.match_number.as_str(),
                s.day.as_str(),
                s.date.as_deref(),
                s.time.as_str()
            ),
            ("42", "7", Some("2027-03-14"), "19:45")
        );
        assert_eq!(
            (s.city.as_str(), s.hall.as_str()),
            ("Harbourtown", "Quayside Arena")
        );
        assert_eq!(s.competition, "Harbour League - Men");
        let [home, away] = &s.teams;
        assert_eq!(
            (home.name.as_str(), home.letter.as_str()),
            ("Harbour City", "B")
        );
        assert_eq!(
            (away.name.as_str(), away.letter.as_str()),
            ("Northgate", "A")
        );

        assert_eq!(home.players.len(), 8, "seven and the libero");
        let p2 = &home.players[1];
        assert_eq!(
            (
                p2.number,
                p2.last_name.as_str(),
                p2.first_name.as_str(),
                p2.id.as_str()
            ),
            (2, "PLAYER2", "Home", "N002")
        );
        assert!(p2.captain && !home.players[0].captain);
        assert!(home.players.iter().any(|p| p.number == 9 && p.libero));
        assert_eq!(away.players.len(), 7);
        assert_eq!(home.staff.len(), 1);
        assert_eq!(
            (
                home.staff[0].role.as_str(),
                home.staff[0].last_name.as_str()
            ),
            ("C", "Marlow")
        );
        assert_eq!(s.referees, ["Okafor Tess"]);
        assert_eq!(s.scorer, "Vance Ida");

        assert_eq!(s.sets.len(), 2);
        let one = &s.sets[0];
        assert_eq!(
            one.lineups,
            [vec![1, 2, 3, 4, 5, 6], vec![11, 12, 13, 14, 15, 16]]
        );
        assert_eq!(one.served_first, Some(0), "Northgate's half says R");
        assert_eq!(one.start, "19:45");
        assert_eq!(one.score, Some([21, 25]), "by team: home first");
        assert_eq!(one.minutes, Some(24));
        assert_eq!(
            one.substitutions,
            [Substitution {
                team: 0,
                position: 3,
                player_out: 3,
                player_in: 7,
                score_in: Some([8, 5]),
                score_back: Some([20, 18]),
            }]
        );
        let two = &s.sets[1];
        assert_eq!(two.served_first, Some(0));
        assert_eq!(two.score, Some([25, 19]));
        assert!(two.substitutions.is_empty());
    }

    #[test]
    fn rejects_other_pdfs() {
        assert!(matches!(
            parse(&Page::default()),
            Err(Error::NotScoresheet(_))
        ));
    }

    #[test]
    fn names_dates_and_times() {
        assert_eq!(
            name_parts("WYNTER DE Q' (N9177) ALDO"),
            ("WYNTER DE Q'".into(), "ALDO".into(), "N9177".into())
        );
        assert_eq!(
            name_parts("Marlow Dennis"),
            ("Marlow".into(), "Dennis".into(), String::new())
        );
        assert_eq!(iso_date("3 Sep 2026").as_deref(), Some("2026-09-03"));
        assert_eq!(iso_date("Saturday"), None);
        assert_eq!(clock("09.05.00"), "09:05");
        assert_eq!(pair("21:17"), Some([21, 17]));
    }
}
