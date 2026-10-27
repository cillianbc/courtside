//! Scoresheets (issue #57) with a made-up sheet: a match made from it, and
//! a coded set checked against it.

#![allow(clippy::expect_used, clippy::unwrap_used)]

use courtside_console::snapshot::Side;
use courtside_console::{PlayerDetail, Session, SheetChoice, TeamForm};
use vb_grammar::Team;
use vb_scoresheet::{Scoresheet, SheetPlayer, SheetSet, SheetStaff, SheetTeam, Substitution};

fn players(numbers: impl IntoIterator<Item = u8>, last: &str) -> Vec<SheetPlayer> {
    numbers
        .into_iter()
        .map(|n| SheetPlayer {
            number: n,
            last_name: format!("{last}{n}"),
            first_name: "Made Up".into(),
            id: format!("N9{n:03}"),
            ..SheetPlayer::default()
        })
        .collect()
}

fn sheet() -> Scoresheet {
    let mut home = players(1..=7, "HOME");
    home.push(SheetPlayer {
        number: 9,
        last_name: "KEEPER".into(),
        libero: true,
        ..SheetPlayer::default()
    });
    Scoresheet {
        source: "eScoresheet 2027.03.01".into(),
        match_number: "42".into(),
        date: Some("2027-03-14".into()),
        time: "19:45".into(),
        city: "Harbourtown".into(),
        hall: "Quayside Arena".into(),
        teams: [
            SheetTeam {
                name: "Harbour City".into(),
                letter: "B".into(),
                players: home,
                staff: Vec::new(),
            },
            SheetTeam {
                name: "Northgate".into(),
                letter: "A".into(),
                players: players(11..=16, "AWAY"),
                staff: vec![SheetStaff {
                    role: "C".into(),
                    last_name: "Brand".into(),
                    first_name: "Ilse".into(),
                    id: String::new(),
                }],
            },
        ],
        sets: vec![SheetSet {
            number: 1,
            served_first: Some(0),
            lineups: [vec![1, 2, 3, 4, 5, 6], vec![11, 12, 13, 14, 15, 16]],
            substitutions: vec![Substitution {
                team: 0,
                position: 3,
                player_out: 3,
                player_in: 7,
                score_in: Some([2, 1]),
                score_back: Some([5, 5]),
            }],
            score: Some([25, 23]),
            ..SheetSet::default()
        }],
        referees: vec!["Okafor Tess".into()],
        ..Scoresheet::default()
    }
}

#[test]
fn a_match_from_a_scoresheet_is_checked_against_it() {
    let tmp = tempfile::tempdir().unwrap();
    let mut session = Session::new();
    session
        .create_season(&tmp.path().join("season.courtside"), "2026/27", 1)
        .unwrap();
    // Harbour City is in the season with 1–6; the sheet adds 7 and libero 9.
    let roster: Vec<PlayerDetail> = (1..=6)
        .map(|n| PlayerDetail {
            id: String::new(),
            number: n,
            code: String::new(),
            last_name: format!("Home{n}"),
            first_name: String::new(),
            nickname: String::new(),
            role: (n == 1).then_some(5),
            libero: false,
            captain: false,
            foreign: false,
            birth_date: None,
            height_cm: None,
            weight_kg: None,
            reach_cm: None,
        })
        .collect();
    let form = TeamForm {
        name: "Harbour City".into(),
        ..TeamForm::default()
    };
    let harbour = session.save_team(None, &form, &roster, 2).unwrap();

    let s = sheet();
    let preview = courtside_console::sheet::preview(s.clone(), &[]);
    assert_eq!(
        preview.teams[0].team_id, None,
        "an empty season knows no one"
    );

    let choice = SheetChoice {
        team_ids: [Some(harbour.id.clone()), None],
    };
    let snap = session.new_match_from_sheet(&s, &choice, 3).unwrap();
    assert_eq!(snap.teams.home.name, "Harbour City");
    assert_eq!(snap.teams.visiting.name, "Northgate");
    let numbers: Vec<u8> = snap.teams.home.players.iter().map(|p| p.number).collect();
    assert_eq!(
        numbers,
        [1, 2, 3, 4, 5, 6, 7, 9],
        "the sheet's 7 and libero 9 added"
    );
    assert!(
        snap.teams
            .home
            .players
            .iter()
            .any(|p| p.number == 9 && p.libero)
    );
    assert_eq!(session.match_venue().unwrap().hall, "Quayside Arena");
    let teams = session.teams().unwrap();
    let northgate = teams.iter().find(|t| t.name == "Northgate").unwrap();
    assert_eq!(
        northgate.staff.coach.last_name, "Brand",
        "a new team takes the sheet's coach"
    );
    assert_eq!(
        northgate.players[0].code, "N9011",
        "and its licence numbers as codes"
    );

    // The lineup screen's sixes.
    let summary = snap.sheet.unwrap();
    assert_eq!(summary.sets[0].sixes.home, [1, 2, 3, 4, 5, 6]);
    assert_eq!(summary.sets[0].serving, Some(Side::Home));

    // Set 1 coded: an ace, then 7 on for 3 at 1:0 (the sheet says 2:1), and
    // 3 never back.
    session
        .set_lineup(Team::Home, [1, 2, 3, 4, 5, 6], 1, None)
        .unwrap();
    session
        .set_lineup(Team::Visiting, [11, 12, 13, 14, 15, 16], 11, None)
        .unwrap();
    session.commit("*01SH#", None).unwrap();
    session.commit("*c03:07", None).unwrap();
    let check = session.scoresheet_check().unwrap().unwrap();
    assert_eq!(check.sets, [1]);
    assert_eq!(
        check.differences,
        [
            "Set 1: the sheet says 25:23, coded 1:0 (Harbour City first)",
            "Set 1: Harbour City 3 → 7 at 2:1 on the sheet, coded at 1:0",
            "Set 1: Harbour City 7 → 3 at 5:5 on the sheet isn't coded",
        ]
    );
}
