//! The local scoresheet samples (`testdata/scoresheets/*.pdf`, never
//! committed: they name players and officials). Each must read, and what it
//! reads must hold together. `COURTSIDE_SHEET_DETAIL=1` prints a summary.
//!
//! Skips when the folder is absent unless `COURTSIDE_REQUIRE_CORPUS=1`.

#![allow(clippy::expect_used, clippy::unwrap_used)]

use std::path::Path;

#[test]
fn samples_read_and_hold_together() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../testdata/scoresheets");
    let Ok(entries) = std::fs::read_dir(&dir) else {
        assert!(
            std::env::var("COURTSIDE_REQUIRE_CORPUS").as_deref() != Ok("1"),
            "corpus required but {} is missing",
            dir.display()
        );
        return;
    };
    let mut read = 0;
    let mut problems = Vec::new();
    for e in entries {
        let path = e.unwrap().path();
        if path.extension().and_then(|x| x.to_str()) != Some("pdf") {
            continue;
        }
        let name = path.file_name().unwrap().to_string_lossy().into_owned();
        let sheet = match vb_scoresheet::read_pdf(&std::fs::read(&path).unwrap()) {
            Ok(s) => s,
            Err(e) => {
                problems.push(format!("{name}: {e}"));
                continue;
            }
        };
        read += 1;
        let mut bad = |m: String| problems.push(format!("{name}: {m}"));
        if sheet.date.is_none() || sheet.hall.is_empty() || sheet.match_number.is_empty() {
            bad(format!(
                "header: date {:?}, hall {:?}, match {:?}",
                sheet.date, sheet.hall, sheet.match_number
            ));
        }
        for (i, t) in sheet.teams.iter().enumerate() {
            if t.name.is_empty() || t.letter.is_empty() || t.players.len() < 6 {
                bad(format!(
                    "team {i}: {:?} {:?}, {} players",
                    t.name,
                    t.letter,
                    t.players.len()
                ));
            }
            if !t.players.iter().any(|p| p.libero) {
                bad(format!("team {i}: no libero"));
            }
        }
        let (mut won, mut sets) = ([0, 0], 0);
        for set in &sheet.sets {
            sets += 1;
            for (i, six) in set.lineups.iter().enumerate() {
                let on_list = |n: &u8| sheet.teams[i].players.iter().any(|p| p.number == *n);
                if six.len() != 6 || !six.iter().all(on_list) {
                    bad(format!("set {}: team {i} six {six:?}", set.number));
                }
            }
            for sub in &set.substitutions {
                let roster = &sheet.teams[sub.team].players;
                if !roster.iter().any(|p| p.number == sub.player_in) || sub.score_in.is_none() {
                    bad(format!("set {}: substitution {sub:?}", set.number));
                }
            }
            if set.served_first.is_none() {
                bad(format!("set {}: no first server", set.number));
            }
            match set.score {
                Some([a, b]) if a != b => won[usize::from(b > a)] += 1,
                other => bad(format!("set {}: score {other:?}", set.number)),
            }
        }
        if won.iter().max() != Some(&3) || sets < 3 {
            bad(format!("sets won {won:?} in {sets} sets"));
        }
        if std::env::var("COURTSIDE_SHEET_DETAIL").is_ok() {
            eprintln!(
                "{name}: {} · {} · {:?} {} · {} · {} {} v {} {} · captains {:?}/{:?} · liberos {:?}/{:?} · staff {}/{} · refs {}",
                sheet.source,
                sheet.match_number,
                sheet.date,
                sheet.time,
                sheet.hall,
                sheet.teams[0].name,
                sheet.teams[0].letter,
                sheet.teams[1].name,
                sheet.teams[1].letter,
                sheet.teams[0]
                    .players
                    .iter()
                    .filter(|p| p.captain)
                    .map(|p| p.number)
                    .collect::<Vec<_>>(),
                sheet.teams[1]
                    .players
                    .iter()
                    .filter(|p| p.captain)
                    .map(|p| p.number)
                    .collect::<Vec<_>>(),
                sheet.teams[0]
                    .players
                    .iter()
                    .filter(|p| p.libero)
                    .map(|p| p.number)
                    .collect::<Vec<_>>(),
                sheet.teams[1]
                    .players
                    .iter()
                    .filter(|p| p.libero)
                    .map(|p| p.number)
                    .collect::<Vec<_>>(),
                sheet.teams[0].staff.len(),
                sheet.teams[1].staff.len(),
                sheet.referees.len(),
            );
            for set in &sheet.sets {
                eprintln!(
                    "  set {} {}–{} {:?}min score {:?} first serve {:?} sixes {:?} subs {:?}",
                    set.number,
                    set.start,
                    set.end,
                    set.minutes,
                    set.score,
                    set.served_first,
                    set.lineups,
                    set.substitutions
                        .iter()
                        .map(|s| format!(
                            "t{} {}→{} pos{} {:?}/{:?}",
                            s.team, s.player_out, s.player_in, s.position, s.score_in, s.score_back
                        ))
                        .collect::<Vec<_>>()
                );
            }
        }
    }
    assert!(read > 0 || !problems.is_empty(), "no samples");
    assert!(problems.is_empty(), "{}", problems.join("\n"));
}
