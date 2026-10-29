//! The extended ("advanced") codes as a scout types them, VolleyStation-
//! style, in a match with the season's default tables: what each becomes
//! and how it reads back. These are the examples in the user guide's
//! "Advanced codes" page; a change here means the page needs it too.

#![allow(clippy::expect_used, clippy::unwrap_used)]

use courtside_console::{PlayerDetail, Session, TeamForm};
use vb_grammar::Team;

/// A season with its default tables; home 1–7 (1 the setter), visiting
/// 11–17 (11 the setter); both sixes in, home to serve.
fn match_from_defaults() -> (Session, tempfile::TempDir) {
    let dir = tempfile::tempdir().expect("tempdir");
    let mut session = Session::new();
    session
        .create_season(&dir.path().join("s.courtside"), "2026/27", 1)
        .expect("season");
    let roster = |base: u8| -> Vec<PlayerDetail> {
        (0..7)
            .map(|i| PlayerDetail {
                id: String::new(),
                number: base + i,
                code: String::new(),
                last_name: format!("Player{}", base + i),
                first_name: String::new(),
                nickname: String::new(),
                role: Some(match i {
                    0 => 5,
                    2 => 4,
                    6 => 1,
                    _ => 2,
                }),
                libero: i == 6,
                captain: false,
                foreign: false,
                birth_date: None,
                height_cm: None,
                weight_kg: None,
                reach_cm: None,
            })
            .collect()
    };
    let form = |name: &str| TeamForm {
        name: name.into(),
        ..TeamForm::default()
    };
    let home = session
        .save_team(None, &form("Harbour City"), &roster(1), 2)
        .expect("home");
    let away = session
        .save_team(None, &form("Northgate"), &roster(11), 3)
        .expect("away");
    session
        .new_match(&home.id, &away.id, None, None, 4)
        .expect("match");
    session
        .set_lineup(Team::Home, [1, 2, 3, 4, 5, 6], 1, None)
        .expect("home six");
    session
        .set_lineup(Team::Visiting, [11, 12, 13, 14, 15, 16], 11, None)
        .expect("away six");
    (session, dir)
}

/// The last `n` codes `line` becomes, with their read-back.
fn last(session: &Session, line: &str, n: usize) -> Vec<(String, String)> {
    let p = session.preview(line).expect("preview");
    assert!(
        p.codes.iter().all(|r| r.error.is_none()),
        "{line}: {:?}",
        p.codes
    );
    let rows: Vec<(String, String)> = p
        .codes
        .iter()
        .map(|r| (r.text.clone(), r.meaning.clone()))
        .collect();
    rows[rows.len() - n..].to_vec()
}

fn pairs(v: &[(&str, &str)]) -> Vec<(String, String)> {
    v.iter()
        .map(|(a, b)| ((*a).to_owned(), (*b).to_owned()))
        .collect()
}

/// Every example on the "Advanced codes" page: typed, written, read back.
#[test]
fn advanced_codes_page_examples() {
    let (session, _dir) = match_from_defaults();
    let s = &session;
    let one = |line: &str, text: &str, meaning: &str| {
        assert_eq!(last(s, line, 1), pairs(&[(text, meaning)]), "{line}");
    };
    // Serve: how a serve error ended, a net cord.
    one(
        "1SM=O",
        "*01SM=~~~~~~~~O",
        "Harbour City #1 serve, medium, error, out long, loses the point",
    );
    one(
        "1SH=L",
        "*01SH=~~~~~~~~L",
        "Harbour City #1 serve, high, error, out left, loses the point",
    );
    one(
        "1SH-N",
        "*01SH-~~~~~~~~N",
        "Harbour City #1 serve, high, poor, off the net",
    );
    one(
        "1SQ#",
        "*01SQ#",
        "Harbour City #1 serve, quick, ace, wins the point",
    );
    // Reception: body position (VolleyStation's Z is overhand), formation,
    // and on an error, how it went wrong.
    one(
        "1SH+ a12R+Z",
        "a12RH+~~~~~~O",
        "Northgate #12 reception, high, positive, overhand",
    );
    one(
        "1SH+ a12R-W",
        "a12RH-~~~~~~W",
        "Northgate #12 reception, high, poor, low",
    );
    one(
        "1SH+ a12R#L3",
        "a12RH#~~~~~~L3",
        "Northgate #12 reception, high, perfect, on the left, three receiving, the left one",
    );
    one(
        "1SH+ a12R=Z",
        "a12RH=~~~~~~~~Z",
        "Northgate #12 reception, high, error, referee call, loses the point",
    );
    // Set: a call alone is the setter's set; hands; a set error.
    assert_eq!(
        last(s, "1SH+ a12R# aK1 a13X1#", 2),
        pairs(&[
            ("a11EH+K1C", "Northgate #11 set, high, positive, call K1"),
            (
                "a13AQ#X1~3",
                "Northgate #13 attack, quick, kill, call X1, wins the point"
            ),
        ])
    );
    one(
        "1SH+ a12R# aK1#",
        "a11EH#K1",
        "Northgate #11 set, high, perfect, call K1",
    );
    one(
        "1SH+ a12R# a11K1#2",
        "a11EH#K1~~~~2",
        "Northgate #11 set, high, perfect, call K1, two hands",
    );
    one(
        "1SH+ a12R# a11K1=5Z",
        "a11EH=K1~~~~5~Z",
        "Northgate #11 set, high, error, call K1, underhand, referee call, loses the point",
    );
    // Attacks on the second or first touch: no set before them.
    assert_eq!(
        last(s, "1SH+ a12R+ a11PP#", 2),
        pairs(&[
            ("a12RH+", "Northgate #12 reception, high, positive"),
            (
                "a11AO#PP~3",
                "Northgate #11 attack, other, kill, call PP, wins the point"
            ),
        ])
    );
    one(
        "1SH+ a12R+ a14P2#",
        "a14AO#P2~3",
        "Northgate #14 attack, other, kill, call P2, wins the point",
    );
    one(
        "1SH+ a12R/ 3PR#",
        "*03AO#PR~4",
        "Harbour City #3 attack, other, kill, call PR, wins the point",
    );
    // Attack: how it was hit, blockers, how it ended.
    one(
        "1SH+ a12R+ a14X5#T2",
        "a14AT#X5~4~~T2",
        "Northgate #14 attack, tense, kill, call X5, tip, two blockers, wins the point",
    );
    one(
        "1SH+ a12R+ a14X5=H1O",
        "a14AT=X5~4~~H1O",
        "Northgate #14 attack, tense, error, call X5, hard hit, one blocker, out long, loses the point",
    );
    one(
        "1SH+ a12R+ a14X5=N",
        "a14AT=X5~4~~~~N",
        "Northgate #14 attack, tense, error, call X5, in the net, loses the point",
    );
    one(
        "1SH+ a12R+ a14X5=I",
        "a14AT=X5~4~~~~I",
        "Northgate #14 attack, tense, error, call X5, net touch, loses the point",
    );
    one(
        "1SH+ a12R+ a14X5=A",
        "a14AT=X5~4~~~~A",
        "Northgate #14 attack, tense, error, call X5, antenna, loses the point",
    );
    one(
        "1SH+ a12R+ a14X5#S",
        "a14AT#X5~4~~~~S",
        "Northgate #14 attack, tense, kill, call X5, off the block, out at the side, wins the point",
    );
    one(
        "1SH+ a12R+ a14X5#X",
        "a14AT#X5~4~~~~X",
        "Northgate #14 attack, tense, kill, call X5, straight to the floor, wins the point",
    );
    // Block after a dot: the blocker's zone mirrors the attack's.
    assert_eq!(
        last(s, "1SH+ a12R+ a14X5/.3#", 2),
        pairs(&[
            (
                "a14AT/X5~4",
                "Northgate #14 attack, tense, blocked, call X5, loses the point"
            ),
            (
                "*03BT#~~~2",
                "Harbour City #3 block, tense, stuff block, wins the point"
            ),
        ])
    );
    one(
        "1SH+ a12R+ a14X5!.3+A2",
        "*03BT+~~~2~~A2",
        "Harbour City #3 block, tense, positive, assist, two blockers",
    );
    one(
        "1SH+ a12R+ a14X5#.3=N",
        "*03BT=~~~2~~~~N",
        "Harbour City #3 block, tense, error, hands and net, loses the point",
    );
    // Dig: after a dot, or typed; the situation.
    one(
        "1SH+ a12R+ a14X5-.5D+S",
        "*05DT+~~~4~~S",
        "Harbour City #5 dig, tense, positive, on a spike",
    );
    one(
        "1SH+ a12R+ a14X5+ 2D+B",
        "*02DT+~~~~~~B",
        "Harbour City #2 dig, tense, positive, after the block",
    );
    one(
        "1SH+ a12R+ a14X5- 2D#C",
        "*02DT#~~~~~~C",
        "Harbour City #2 dig, tense, perfect, spike cover",
    );
    one(
        "1SH+ a12R+ a14X5- 2D-E",
        "*02DT-~~~~~~E",
        "Harbour City #2 dig, tense, poor, emergency",
    );
}
