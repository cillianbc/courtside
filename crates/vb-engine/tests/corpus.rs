//! Phase 2 task 3: the fold reproduces the `[3SCOUT]` columns DataVolley
//! and VolleyStation derive themselves (set 8, setter positions 9–10,
//! players on court 14–25) on every line of the corpus, except lines where
//! the file contradicts itself (listed in `ACCEPTED`, explained in
//! FORMAT-NOTES "Engine fold").
//!
//! Skips when `testdata/` is absent unless `COURTSIDE_REQUIRE_CORPUS=1`.

#![allow(clippy::expect_used)]

use std::collections::BTreeMap;
use std::path::Path;

use dvw_codec::Dvw;
use serde::Deserialize;
use vb_engine::{EventKind, IdGen, Match, fold};

#[derive(Deserialize)]
struct Manifest {
    file: Vec<Entry>,
}

#[derive(Deserialize)]
struct Entry {
    path: String,
}

/// (file name prefix, column group, number of lines) where the file's own
/// columns contradict its codes or player table; see FORMAT-NOTES.
const ACCEPTED: &[(&str, &str, usize)] = &[
    // Setter position written as 0 for ten lines with no code changing it.
    ("&corpus-file", "setter position", 10),
    // Home court columns don't match the player table's set 1 lineup.
    ("&corpus-file", "home court", 274),
];

fn corpus() -> Option<(std::path::PathBuf, Manifest)> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../testdata");
    let Ok(text) = std::fs::read_to_string(root.join("manifest.toml")) else {
        assert!(
            std::env::var("COURTSIDE_REQUIRE_CORPUS").as_deref() != Ok("1"),
            "corpus required but missing"
        );
        eprintln!("skipping: no corpus");
        return None;
    };
    Some((root, toml::from_str(&text).expect("parse manifest")))
}

#[test]
fn fold_reproduces_the_files_own_columns() {
    let Some((root, manifest)) = corpus() else {
        return;
    };
    let mut mismatches: BTreeMap<(String, &str), Vec<String>> = BTreeMap::new();
    let mut checked = 0usize;
    for entry in manifest.file.iter().filter(|e| e.path.ends_with(".dvw")) {
        let bytes = std::fs::read(root.join(&entry.path)).expect("read corpus file");
        let dvw = Dvw::parse(&bytes).expect("parse corpus file");
        let volleystation = dvw.header.get("GENERATOR-PRG") == Some("VolleyStation");
        let m = Match::from_dvw(&dvw, &mut IdGen::new(0));
        let events = m.log.visible();
        let folded = fold(&m);
        let name = entry
            .path
            .rsplit('/')
            .next()
            .unwrap_or(&entry.path)
            .to_owned();
        for (i, line) in dvw.scout.iter().enumerate() {
            checked += 1;
            let mut st = &folded.states[i];
            // VolleyStation writes a rotation line with the substitutions
            // that follow it already applied.
            if volleystation && events[i].kind() == EventKind::SetterPosition {
                let mut j = i;
                while events
                    .get(j + 1)
                    .is_some_and(|e| e.kind() == EventKind::Substitution)
                {
                    j += 1;
                }
                st = &folded.states[j];
            }
            let mut check = |group: &'static str, ok: bool, detail: String| {
                if !ok {
                    mismatches
                        .entry((name.clone(), group))
                        .or_default()
                        .push(format!("line {i} {:?}: {detail}", line.code()));
                }
            };
            check(
                "set",
                line.set_number() == Some(st.set),
                format!("file {:?} engine {}", line.set_number(), st.set),
            );
            let file_sp = (line.home_setter_position(), line.visiting_setter_position());
            let ours = (st.setter_position.home, st.setter_position.visiting);
            let comparable = |f: Option<u8>, o: Option<u8>| o.is_none() || f == o;
            check(
                "setter position",
                comparable(file_sp.0, ours.0) && comparable(file_sp.1, ours.1),
                format!("file {file_sp:?} engine {ours:?}"),
            );
            check(
                "home court",
                line.home_players() == st.court.home,
                format!("file {:?} engine {:?}", line.home_players(), st.court.home),
            );
            check(
                "visiting court",
                line.visiting_players() == st.court.visiting,
                format!(
                    "file {:?} engine {:?}",
                    line.visiting_players(),
                    st.court.visiting
                ),
            );
        }
    }
    assert!(checked > 0, "no scout lines");
    let mut unexpected = Vec::new();
    for ((file, group), lines) in &mismatches {
        let accepted = ACCEPTED
            .iter()
            .any(|(f, g, n)| file.starts_with(f) && g == group && *n == lines.len());
        eprintln!(
            "{file} {group}: {} lines{}",
            lines.len(),
            if accepted { " (accepted)" } else { "" }
        );
        if !accepted {
            unexpected.push(format!(
                "{file} {group}: {} lines, first: {}",
                lines.len(),
                lines
                    .iter()
                    .take(3)
                    .cloned()
                    .collect::<Vec<_>>()
                    .join(" | ")
            ));
        }
    }
    eprintln!("{checked} lines checked");
    assert!(
        unexpected.is_empty(),
        "fold differs from the file:\n{}",
        unexpected.join("\n")
    );
}

/// PRD: a full match (~1,500 events) folds in under 5 ms. Timed only in
/// release builds (`cargo test --release`).
#[test]
fn fold_is_fast_enough_to_run_on_every_change() {
    if cfg!(debug_assertions) {
        eprintln!("skipping: debug build");
        return;
    }
    let Some((root, manifest)) = corpus() else {
        return;
    };
    let mut slowest = std::time::Duration::ZERO;
    for entry in manifest.file.iter().filter(|e| e.path.ends_with(".dvw")) {
        let bytes = std::fs::read(root.join(&entry.path)).expect("read corpus file");
        let dvw = Dvw::parse(&bytes).expect("parse corpus file");
        let m = Match::from_dvw(&dvw, &mut IdGen::new(0));
        for _ in 0..5 {
            let start = std::time::Instant::now();
            let folded = fold(&m);
            slowest = slowest.max(start.elapsed());
            assert_eq!(folded.states.len(), dvw.scout.len());
        }
    }
    eprintln!("slowest fold: {slowest:?}");
    assert!(slowest < std::time::Duration::from_millis(5), "{slowest:?}");
}
