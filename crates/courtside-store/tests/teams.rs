//! Teams, rosters and match rosters, with made-up teams and players.

#![allow(clippy::expect_used, clippy::unwrap_used)]

use courtside_store::{PlayerInput, Season, TeamInput};

fn player(number: u8, last: &str, first: &str) -> PlayerInput {
    PlayerInput {
        number,
        last_name: last.into(),
        first_name: first.into(),
        ..PlayerInput::default()
    }
}

fn team(season: &mut Season, name: &str) -> String {
    season
        .create_team(
            &TeamInput {
                name: name.into(),
                coach: "Coach".into(),
                ..TeamInput::default()
            },
            1,
        )
        .expect("team")
}

fn six(season: &mut Season, team_id: &str, base: u8) {
    let rows: Vec<PlayerInput> = (0..7)
        .map(|i| {
            let mut p = player(base + i, &format!("Last{}", base + i), "First");
            p.libero = i == 6;
            p.role = Some(if i == 6 {
                1
            } else if i == 0 {
                5
            } else {
                2
            });
            p
        })
        .collect();
    season.save_roster(team_id, &rows, 2).expect("roster");
}

#[test]
fn a_new_team_gets_codes_for_itself_and_its_players() {
    let mut season = Season::in_memory("s", 1).expect("season");
    let id = team(&mut season, "Harbour City VC");
    season
        .save_roster(
            &id,
            &[
                player(1, "Ardent", "Mara"),
                player(4, "Ardent", "Mattie"),
                PlayerInput {
                    code: "OWN-1".into(),
                    ..player(7, "Corrin", "Tess")
                },
            ],
            2,
        )
        .expect("roster");
    let t = season.team(&id).expect("team");
    assert_eq!(t.code, "HCV");
    let codes: Vec<&str> = t.players.iter().map(|p| p.code.as_str()).collect();
    assert_eq!(codes, ["ARD-MAR", "ARD-MAT", "OWN-1"]);
    assert_eq!(t.matches, 0);
}

#[test]
fn roster_mistakes_are_refused_and_change_nothing() {
    let mut season = Season::in_memory("s", 1).expect("season");
    let a = team(&mut season, "Harbour City");
    let b = team(&mut season, "Northgate");
    season
        .save_roster(&a, &[player(1, "Ardent", "Mara")], 2)
        .expect("roster");
    fn season_err(season: &mut Season, team: &str, rows: &[PlayerInput]) -> String {
        season.save_roster(team, rows, 3).unwrap_err().to_string()
    }
    assert_eq!(
        season_err(&mut season, &b, &[player(2, "A", ""), player(2, "B", "")]),
        "#2 is on the roster twice"
    );
    assert_eq!(
        season_err(
            &mut season,
            &b,
            &[PlayerInput {
                code: "ARD-MAR".into(),
                ..player(5, "Other", "")
            }]
        ),
        "#5: code ARD-MAR belongs to another player"
    );
    assert_eq!(
        season_err(&mut season, &b, &[player(100, "A", "")]),
        "#100: shirt numbers go from 0 to 99"
    );
    assert_eq!(
        season_err(&mut season, &b, &[player(3, " ", "")]),
        "#3: enter a last name"
    );
    assert!(season.team(&b).expect("team").players.is_empty());
    // Team names and codes are unique too.
    let dup = season.create_team(
        &TeamInput {
            name: "Northgate".into(),
            ..TeamInput::default()
        },
        4,
    );
    assert_eq!(
        dup.unwrap_err().to_string(),
        "there is already a team called Northgate"
    );
}

#[test]
fn players_keep_their_id_across_edits_and_can_swap_numbers() {
    let mut season = Season::in_memory("s", 1).expect("season");
    let id = team(&mut season, "Harbour City");
    season
        .save_roster(
            &id,
            &[player(1, "Ardent", "Mara"), player(2, "Bellamy", "Jo")],
            2,
        )
        .expect("roster");
    let before = season.team(&id).expect("team").players;
    let mut rows: Vec<PlayerInput> = before
        .iter()
        .map(|p| PlayerInput {
            id: Some(p.id.clone()),
            number: p.number,
            code: p.code.clone(),
            last_name: p.last_name.clone(),
            first_name: p.first_name.clone(),
            ..PlayerInput::default()
        })
        .collect();
    rows[0].number = 2;
    rows[1].number = 1;
    rows[1].captain = true;
    season.save_roster(&id, &rows, 3).expect("swap");
    let after = season.team(&id).expect("team").players;
    let ardent = after
        .iter()
        .find(|p| p.last_name == "Ardent")
        .expect("Ardent");
    assert_eq!(
        (ardent.id.as_str(), ardent.number),
        (before[0].id.as_str(), 2)
    );
    assert!(after.iter().find(|p| p.number == 1).expect("1").captain);
    // Leaving a row out takes the player off the roster.
    season.save_roster(&id, &rows[..1], 4).expect("drop one");
    assert_eq!(season.team(&id).expect("team").players.len(), 1);
}

#[test]
fn a_match_keeps_its_roster_when_the_team_changes() {
    let mut season = Season::in_memory("s", 1).expect("season");
    let a = team(&mut season, "Harbour City");
    let b = team(&mut season, "Northgate");
    six(&mut season, &a, 1);
    six(&mut season, &b, 11);
    let m = season
        .create_match(&a, &b, Some("2026-10-15"), 5)
        .expect("match");
    // A new player joins the team after the match.
    let mut rows: Vec<PlayerInput> = season
        .team(&a)
        .expect("team")
        .players
        .iter()
        .map(|p| PlayerInput {
            id: Some(p.id.clone()),
            number: p.number,
            last_name: p.last_name.clone(),
            code: p.code.clone(),
            ..PlayerInput::default()
        })
        .collect();
    rows.push(player(20, "Newcomer", "Ali"));
    season.save_roster(&a, &rows, 6).expect("roster");
    let (home, _) = season.match_roster(m).expect("match roster");
    assert_eq!(home.len(), 7, "the match keeps its own roster");
    assert!(home.iter().any(|p| p.libero && p.number == 7));

    // A late addition for this match only.
    let mut late: Vec<PlayerInput> = home
        .iter()
        .map(|p| PlayerInput {
            id: Some(p.id.clone()),
            number: p.number,
            code: p.code.clone(),
            last_name: p.last_name.clone(),
            libero: p.libero,
            ..PlayerInput::default()
        })
        .collect();
    late.push(player(30, "Guest", "Sam"));
    season
        .save_match_roster(m, true, &late)
        .expect("match roster");
    let (home, _) = season.match_roster(m).expect("match roster");
    assert_eq!(home.len(), 8);
    assert_eq!(
        season.team(&a).expect("team").players.len(),
        8,
        "the team is unchanged"
    );

    // The export carries the match roster, codes and roles.
    let text = String::from_utf8_lossy(&season.export_dvw(m, false).expect("export")).into_owned();
    let row = text
        .lines()
        .find(|l| l.starts_with("0;7;"))
        .expect("home libero");
    let f: Vec<&str> = row.split(';').collect();
    assert_eq!((f[8], f[12], f[13]), ("LAS-FIR-7", "L", "1"), "{row}");
    assert!(text.lines().any(|l| l.starts_with("0;30;")));
    assert!(text.contains("\r\nHC;Harbour City;"), "team code");

    assert_eq!(
        season.delete_team(&a).unwrap_err().to_string(),
        "the team plays in 1 match(es); merge it into another team instead"
    );
}

#[test]
fn merging_teams_and_players() {
    let mut season = Season::in_memory("s", 1).expect("season");
    let old = team(&mut season, "Harbour City U20");
    let new = team(&mut season, "Harbour City");
    let other = team(&mut season, "Northgate");
    six(&mut season, &old, 1);
    six(&mut season, &new, 1);
    six(&mut season, &other, 11);
    let m = season.create_match(&old, &other, None, 5).expect("match");

    let old_players = season.team(&old).expect("old").players;
    let new_players = season.team(&new).expect("new").players;
    // #1–#6 are the same people; the libero #7 too; nobody moves across.
    let pairs: Vec<(String, String)> = old_players
        .iter()
        .zip(&new_players)
        .map(|(o, n)| (o.id.clone(), n.id.clone()))
        .collect();
    season.merge_teams(&old, &new, &pairs).expect("merge");
    let teams = season.teams().expect("teams");
    assert_eq!(teams.len(), 2);
    let merged = teams.iter().find(|t| t.id == new).expect("merged");
    assert_eq!((merged.players.len(), merged.matches), (7, 1));
    let (home, _) = season.match_roster(m).expect("roster");
    assert_eq!(
        home[0].id, new_players[0].id,
        "the match points at the kept player"
    );

    // Merging two records of one player.
    let players = season.players().expect("players");
    assert_eq!(players.len(), 14);
    let keep = &new_players[1].id;
    let drop = &players
        .iter()
        .find(|p| p.team_id == other && p.player.number == 11)
        .expect("player")
        .player
        .id;
    season.merge_players(keep, drop).expect("merge players");
    assert_eq!(season.players().expect("players").len(), 13);
    let (_, visiting) = season.match_roster(m).expect("roster");
    assert_eq!(
        &visiting.iter().find(|p| p.number == 11).expect("11").id,
        keep
    );
}

#[test]
fn a_team_and_its_roster_save_together_or_not_at_all() {
    let mut season = Season::in_memory("s", 1).expect("season");
    let input = TeamInput {
        name: "Harbour City".into(),
        ..TeamInput::default()
    };
    let bad = [player(1, "Ardent", ""), player(1, "Bellamy", "")];
    assert!(season.save_team(None, &input, &bad, 2).is_err());
    assert!(
        season.teams().expect("teams").is_empty(),
        "no half-made team"
    );
    let id = season
        .save_team(None, &input, &[player(1, "Ardent", "")], 3)
        .expect("team");
    let renamed = TeamInput {
        name: "Harbour City VC".into(),
        code: "HAR".into(),
        ..input
    };
    season
        .save_team(
            Some(&id),
            &renamed,
            &[player(1, "Ardent", ""), player(2, "Bellamy", "")],
            4,
        )
        .expect("update");
    let t = season.team(&id).expect("team");
    assert_eq!(
        (t.name.as_str(), t.code.as_str(), t.players.len()),
        ("Harbour City VC", "HAR", 2)
    );
}

#[test]
fn team_files_add_a_team_or_fill_in_its_roster() {
    let mut season = Season::in_memory("s", 1).expect("season");
    // VolleyStation-style .sq (made-up team).
    let sq = "DV-Team-2\nHCV\tHarbour City\tMarlow Dennis\tOkafor Tess\tHCV\t1252\n\
              3\tHCV-ARD\tArdent\t\t\t\t\t\tJonah\t2\t\t\tFalse\n\
              7\tHCV-QUI\tQuill\t\t\t\tL\t\tMaya\t1\t\t\tTrue";
    let first = season
        .import_team_file("HCV.sq", sq.as_bytes(), 2)
        .expect("import");
    assert!(first.created);
    assert_eq!((first.players_added, first.players_kept), (2, 0));
    let t = season.team(&first.team_id).expect("team");
    assert_eq!(
        (t.name.as_str(), t.code.as_str(), t.coach.as_str()),
        ("Harbour City", "HCV", "Marlow Dennis")
    );
    assert!(
        t.players
            .iter()
            .any(|p| p.code == "HCV-QUI" && p.libero && p.foreign)
    );

    // The same team as .vst, with one more player: reused, filled in.
    let vst = r#"{"code":"HCV","name":"Harbour City","players":[
        {"code":"HCV-ARD","lastName":"Ardent","firstName":"Jonah","shirtNumber":3,"position":1},
        {"code":"HCV-BRA","lastName":"Brannigan","firstName":"Nell","shirtNumber":11,"position":4,"height":181}]}"#;
    let second = season
        .import_team_file("HCV.vst", vst.as_bytes(), 3)
        .expect("import");
    assert!(!second.created);
    assert_eq!(second.team_id, first.team_id);
    assert_eq!((second.players_added, second.players_kept), (1, 1));
    let t = season.team(&first.team_id).expect("team");
    let b = t.players.iter().find(|p| p.number == 11).expect("#11");
    assert_eq!((b.role, b.height_cm), (Some(5), Some(181)));

    assert_eq!(
        season
            .import_team_file("x.sq", b"hello", 4)
            .unwrap_err()
            .to_string(),
        "not a team file: the first line isn't DV-Team-2"
    );
}

#[test]
fn a_team_exported_and_imported_elsewhere_is_the_same_team() {
    let mut season = Season::in_memory("s", 1).expect("season");
    let id = team(&mut season, "Harbour City");
    six(&mut season, &id, 1);
    let original = season.team(&id).expect("team");
    for vst in [false, true] {
        let bytes = season.export_team_file(&id, vst).expect("export");
        let mut other = Season::in_memory("other", 1).expect("season");
        let name = if vst { "HC.vst" } else { "HC.sq" };
        let r = other.import_team_file(name, &bytes, 2).expect("import");
        assert!(r.created);
        let copy = other.team(&r.team_id).expect("team");
        assert_eq!(
            (copy.name.as_str(), copy.code.as_str()),
            ("Harbour City", "HC")
        );
        let strip = |t: &courtside_store::Team| -> Vec<_> {
            t.players
                .iter()
                .map(|p| {
                    (
                        p.number,
                        p.code.clone(),
                        p.last_name.clone(),
                        p.role,
                        p.libero,
                    )
                })
                .collect()
        };
        assert_eq!(strip(&copy), strip(&original), "{name}");
    }
}

#[test]
fn settings_are_saved_and_each_match_keeps_the_tables_it_started_with() {
    let mut season = Season::in_memory("s", 1).expect("season");
    let mut settings = season.settings().expect("defaults");
    assert_eq!(settings.typing.skill, 'A');
    assert!(settings.tables.combinations.iter().any(|c| c.code == "X5"));
    let a = team(&mut season, "Harbour City");
    let b = team(&mut season, "Northgate");
    six(&mut season, &a, 1);
    six(&mut season, &b, 11);
    let first = season.create_match(&a, &b, None, 5).expect("match");

    settings.typing.evaluations[0] = '-';
    settings.tables.combinations.retain(|c| c.code != "X5");
    settings.tables.winning[3].lose = "=".into(); // attack: only = loses
    season.save_settings(&settings).expect("save");
    assert_eq!(season.settings().expect("load"), settings);
    let second = season.create_match(&a, &b, None, 6).expect("match");

    let export =
        |m| String::from_utf8_lossy(&season.export_dvw(m, false).expect("export")).into_owned();
    let (one, two) = (export(first), export(second));
    assert!(
        one.contains("\r\nX5;4;L;T;Fast set to zone 4;;;;F;;\r\n"),
        "first keeps X5"
    );
    assert!(!two.contains("\r\nX5;"), "second has the new table");
    assert!(one.contains("\r\n=~~~#~~~=~~~~~~~=/~~#~~~"));
    assert!(two.contains("\r\n=~~~#~~~=~~~~~~~=~~~#~~~"));
    assert!(
        season
            .open_match(second)
            .expect("open")
            .game
            .winning_symbols
            .outcome(vb_grammar::Skill::Attack, '/')
            .is_none(),
        "the engine uses the match's winning symbols"
    );
}
