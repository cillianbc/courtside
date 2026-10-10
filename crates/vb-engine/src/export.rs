//! [`Match`] to `.dvw`: the `[3SCOUT]` section regenerated from the event
//! log and the fold, everything else from the file the match came from.
//!
//! Each visible event (comments aside) becomes one line. An event imported
//! from the template, or a correction of one, starts from its source line,
//! so columns the engine doesn't model (phases, coordinates, video) are
//! kept as written. Inserted events start from an empty line. The engine
//! then writes the code and the derived columns: set (8), setter positions
//! (9–10, kept as written where the fold doesn't know them) and players on
//! court (14–25).
//!
//! Not yet regenerated: `[3SET]` scores and `[3TEAMS]` sets won, and the
//! player table's starting zones. They are written as in the template.

use dvw_codec::Dvw;
use dvw_codec::scout::ScoutLine;
use dvw_codec::text::Row;

use crate::event::{Body, MatchEvent};
use crate::fold::{State, fold_events};
use crate::model::Match;

/// Fields in a `[3SCOUT]` line, counting the empty one after the trailing `;`.
const WIDTH: usize = 27;

impl Match {
    /// The match as a `.dvw`, with headers from `template` (normally the
    /// file it was imported from).
    pub fn to_dvw(&self, template: &Dvw) -> Dvw {
        let events = self.log.visible();
        let folded = fold_events(self, &events);
        let mut dvw = template.clone();
        dvw.scout = events
            .iter()
            .zip(&folded.states)
            .filter_map(|(e, st)| line(e, st, template))
            .collect();
        dvw
    }
}

fn line(e: &MatchEvent, st: &State, template: &Dvw) -> Option<ScoutLine> {
    let Body::Code(code) = &e.body else {
        return None;
    };
    let source = e.source_line.and_then(|i| template.scout.get(i));
    let mut row = match source {
        Some(l) => l.0.0.clone(),
        None => {
            let mut row = vec![String::new(); WIDTH];
            if let Some(c) = e.clock {
                row[7] = format!("{:02}.{:02}.{:02}", c.hour, c.minute, c.second);
            }
            if let Some(ms) = e.video_ms {
                row[12] = (ms / 1000).to_string();
            }
            row
        }
    };
    if row.len() < WIDTH {
        row.resize(WIDTH, String::new());
    }
    let num = |n: Option<u8>| n.map(|n| n.to_string()).unwrap_or_default();
    row[0] = code.to_string();
    row[8] = st.set.to_string();
    for (col, sp) in [
        (9, st.setter_position.home),
        (10, st.setter_position.visiting),
    ] {
        if sp.is_some() || source.is_none() {
            row[col] = num(sp);
        }
    }
    for (i, p) in st.court.home.iter().chain(&st.court.visiting).enumerate() {
        row[14 + i] = num(*p);
    }
    Some(ScoutLine(Row(row)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::{ClockTime, IdGen, NewEvent, Producer};
    use vb_grammar::Code;

    const FILE: &str = "[3DATAVOLLEYSCOUT]\r\nFILEFORMAT: 2.0\r\n[3PLAYERS-H]\r\n\
        0;8;1;1;;;;;h8;Placeholder;A;;;5;False;h8;;\r\n0;12;2;2;;;;;h12;Placeholder;B;;;2;False;h12;;\r\n\
        [3SCOUT]\r\n*P08>LUp;;;;;;;15.30.00;1;9;;;;;8;12;;;;;;;;;;;\r\n\
        *z1>LUp;;;;;;;15.30.01;1;1;;;;;8;12;;;;;;;;;;;\r\n\
        *08SH-;;;;;;;15.30.10;1;1;;;;;8;12;;;;;;;;;;;\r\n";

    #[test]
    fn unchanged_match_writes_the_same_file() {
        let dvw = Dvw::parse(FILE.as_bytes()).expect("parse");
        let m = Match::from_dvw(&dvw, &mut IdGen::new(0));
        let out = m.to_dvw(&dvw).to_bytes().expect("bytes");
        // The setter position before the first z code is unknown to the
        // engine, so the file's value (9) stays.
        assert_eq!(String::from_utf8_lossy(&out), FILE);
    }

    #[test]
    fn edits_are_written_with_fresh_derived_columns() {
        let dvw = Dvw::parse(FILE.as_bytes()).expect("parse");
        let mut m = Match::from_dvw(&dvw, &mut IdGen::new(0));
        let mut ids = IdGen::new(1);
        let serve = m.log.visible()[2].id;
        m.log
            .correct(
                &mut ids,
                serve,
                NewEvent::code(Code::parse("*08SM-"), Producer::Keyboard),
            )
            .expect("correct");
        m.log.append(
            &mut ids,
            NewEvent {
                clock: Some(ClockTime {
                    hour: 15,
                    minute: 30,
                    second: 15,
                }),
                video_ms: Some(12_400),
                ..NewEvent::code(Code::parse("*c12:05"), Producer::Keyboard)
            },
        );
        let out = m.to_dvw(&dvw);
        let lines: Vec<String> = out.scout.iter().map(|l| l.0.to_line()).collect();
        assert_eq!(lines[2], "*08SM-;;;;;;;15.30.10;1;1;;;;;8;12;;;;;;;;;;;");
        assert_eq!(lines[3], "*c12:05;;;;;;;15.30.15;1;1;;;12;;8;5;;;;;;;;;;;");
    }
}
