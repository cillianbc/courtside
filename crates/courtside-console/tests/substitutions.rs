//! Regular substitutions in the snapshot (#75): who went off for whom in
//! the set in progress, and who may come on for each player on court.

#![allow(clippy::expect_used, clippy::unwrap_used)]

use courtside_console::Session;
use courtside_console::snapshot::Snapshot;
use vb_grammar::Team;

/// Made-up match: home 1–6 start, 7–10 on the bench; visiting 11–16 start,
/// 17 libero, 18 on the bench.
fn file() -> String {
    let mut s = String::from(
        "[3DATAVOLLEYSCOUT]\r\nFILEFORMAT: 2.0\r\nGENERATOR-PRG: Data Volley\r\n\
        [3MATCH]\r\n04/23/2026;;;;;;;;1252;1;Z;\r\n\
        [3TEAMS]\r\nT1;Harbour City;0;Coach A;;\r\nT2;Northgate;0;Coach B;;\r\n\
        [3SET]\r\nTrue;;;;;;\r\n[3PLAYERS-H]\r\n",
    );
    for n in 1..=10u8 {
        let zone = if n <= 6 { n.to_string() } else { String::new() };
        s.push_str(&format!(
            "0;{n};{n};{zone};;;;;h{n};Home{n};P;;;2;False;h{n};;\r\n"
        ));
    }
    s.push_str("[3PLAYERS-V]\r\n");
    for n in 11..=18u8 {
        let zone = if n <= 16 {
            (n - 10).to_string()
        } else {
            String::new()
        };
        let libero = if n == 17 { "L" } else { "" };
        s.push_str(&format!(
            "1;{n};{n};{zone};;;;;v{n};Away{n};P;;{libero};2;False;v{n};;\r\n"
        ));
    }
    s.push_str("[3SCOUT]\r\n");
    for c in ["*P01>LUp", "*z1>LUp", "aP14>LUp", "az4>LUp"] {
        s.push_str(&format!("{c};;;;;;;;1;;;;;;;;;;;;;;;;;;;\r\n"));
    }
    s
}

fn opened() -> (Session, tempfile::TempDir) {
    let dir = tempfile::tempdir().expect("tempdir");
    let dvw = dir.path().join("m.dvw");
    std::fs::write(&dvw, file()).expect("write");
    let mut session = Session::new();
    session
        .create_season(&dir.path().join("s.courtside"), "s", 1)
        .expect("season");
    let id = session.import_file(&dvw, 1_000).expect("import");
    session.open_match(id, 2_000).expect("open");
    (session, dir)
}

fn pairs(snap: &Snapshot) -> Vec<(u8, u8, &str)> {
    snap.substitution_pairs
        .home
        .iter()
        .map(|p| (p.starter, p.substitute, p.state))
        .collect()
}

/// Who may come on for home's `off`, and why nobody may.
fn options(snap: &Snapshot, off: u8) -> (Vec<u8>, Option<String>) {
    let o = snap
        .substitution_options
        .home
        .iter()
        .find(|o| o.off == off)
        .unwrap_or_else(|| panic!("#{off} is not on court"));
    (o.on.clone(), o.reason.clone())
}

fn row_id(snap: &Snapshot, text: &str) -> String {
    snap.groups
        .iter()
        .flat_map(|g| &g.rows)
        .find(|r| r.text == text)
        .map(|r| r.id.clone())
        .expect(text)
}

fn substitution_issues(snap: &Snapshot) -> Vec<String> {
    snap.issues
        .iter()
        .filter(|i| i.rule == "substitution")
        .map(|i| i.message.clone())
        .collect()
}

#[test]
fn a_substitute_can_only_go_off_for_their_starter() {
    let (mut session, _dir) = opened();
    let snap = session.snapshot().expect("snapshot");
    assert!(pairs(&snap).is_empty());
    assert_eq!(options(&snap, 3).0, [7, 8, 9, 10]);
    // The visiting libero is never offered.
    let visiting = &snap.substitution_options.visiting;
    assert!(visiting.iter().all(|o| o.on == [18]), "{visiting:?}");

    let snap = session.commit("*c03:07", None).expect("sub");
    assert_eq!(pairs(&snap), [(3, 7, "out")]);
    assert_eq!(options(&snap, 7), (vec![3], None));
    // 3 has played: an unchanged starter gets the rest of the bench.
    assert_eq!(options(&snap, 2).0, [8, 9, 10]);

    // The return closes the pair.
    let snap = session.commit("*c07:03", None).expect("return");
    assert_eq!(pairs(&snap), [(3, 7, "back")]);
    let (on, reason) = options(&snap, 3);
    assert!(on.is_empty());
    assert!(reason.expect("reason").contains("off and back"));
    assert_eq!(options(&snap, 2).0, [8, 9, 10]);
    assert!(substitution_issues(&snap).is_empty());

    // A third change of the pair is in the log as a check.
    let snap = session.commit("*c03:08", None).expect("illegal sub");
    assert_eq!(
        substitution_issues(&snap),
        ["player 3 has already gone off and come back on this set"]
    );
    // Its pairs are as they were.
    assert_eq!(pairs(&snap), [(3, 7, "back")]);
}

#[test]
fn deleting_the_return_reopens_the_pair() {
    let (mut session, _dir) = opened();
    session.commit("*c03:07", None).expect("sub");
    let snap = session.commit("*c07:03", None).expect("return");
    assert_eq!(pairs(&snap), [(3, 7, "back")]);

    let snap = session.remove(&row_id(&snap, "*c07:03")).expect("remove");
    assert_eq!(pairs(&snap), [(3, 7, "out")]);
    assert_eq!(options(&snap, 7).0, [3]);

    // Undoing the substitution itself leaves no pair.
    let snap = session.remove(&row_id(&snap, "*c03:07")).expect("remove");
    assert!(pairs(&snap).is_empty());
    assert_eq!(options(&snap, 3).0, [7, 8, 9, 10]);
    assert_eq!(snap.substitutions.home, 0);
}

#[test]
fn pairs_reset_with_the_set() {
    let (mut session, _dir) = opened();
    session.commit("*c03:07", None).expect("sub");
    session.commit("*c07:03", None).expect("return");
    for n in 1..=25 {
        session
            .commit(&format!("*p{n:02}:00"), None)
            .expect("point");
    }
    session
        .set_lineup(Team::Home, [2, 3, 4, 5, 6, 1], 1, None, None)
        .expect("home six");
    let snap = session
        .set_lineup(Team::Visiting, [11, 12, 13, 14, 15, 16], 14, None, None)
        .expect("visiting six");
    assert_eq!(snap.set, 2);
    assert!(pairs(&snap).is_empty());
    assert_eq!(options(&snap, 3).0, [7, 8, 9, 10]);
    assert_eq!(snap.substitutions.home, 0);
}
