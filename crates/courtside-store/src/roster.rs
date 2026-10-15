//! An imported match's own roster, written back into its file's
//! `[3PLAYERS]` rows on open, so the engine and the export see edits made
//! in the console. A row whose player is unchanged is kept byte for byte;
//! a changed one has only those fields rewritten (with DataVolley's Unicode
//! copy when the row has one); a new player gets a row in the file's own
//! style; a player taken off the roster loses their row.

use dvw_codec::Dvw;
use dvw_codec::dvw::Player as Row;
use dvw_codec::text::{Row as Fields, encode_embedded};

use crate::Player;

/// What a file row says about its player, in the roster's terms.
fn read(row: &Row) -> Player {
    let special = row.special_role();
    let role = row.role().map(crate::role_number);
    Player {
        id: String::new(),
        number: row.number().unwrap_or(0),
        code: row.id().trim().to_owned(),
        last_name: row.last_name(),
        first_name: row.first_name(),
        nickname: row.nickname(),
        role,
        libero: special == "L" || role == Some(1),
        captain: special == "C",
        foreign: row.foreign().unwrap_or(false),
        birth_date: None,
        height_cm: None,
    }
}

/// Same player data as far as the file holds it. A file without player
/// ids leaves the code to the season, so codes only count when it has one.
fn same(file: &Player, roster: &Player) -> bool {
    (file.code.is_empty() || file.code == roster.code)
        // The roster keeps names trimmed; files can carry trailing spaces
        // and tabs (FORMAT-NOTES "Field content").
        && file.last_name.trim() == roster.last_name.trim()
        && file.first_name.trim() == roster.first_name.trim()
        && file.nickname.trim() == roster.nickname.trim()
        && file.role == roster.role
        && file.libero == roster.libero
        && file.captain == roster.captain
        && file.foreign == roster.foreign
}

/// Applies the match rosters to `dvw`'s player tables.
pub(crate) fn apply(dvw: &mut Dvw, home: &[Player], visiting: &[Player]) {
    let encoding = dvw.encoding;
    let mut index = dvw
        .players_home
        .iter()
        .chain(&dvw.players_visiting)
        .filter_map(|r| r.0.int::<u32>(2))
        .max()
        .unwrap_or(0);
    for (side, rows, roster) in [
        (0u8, &mut dvw.players_home, home),
        (1, &mut dvw.players_visiting, visiting),
    ] {
        let width = rows.first().map_or(16, |r| r.0.0.len());
        let sorted = rows.windows(2).all(|w| w[0].number() <= w[1].number());
        rows.retain(|r| {
            r.number()
                .is_some_and(|n| roster.iter().any(|p| p.number == n))
        });
        for p in roster {
            match rows.iter_mut().find(|r| r.number() == Some(p.number)) {
                Some(row) => {
                    if !same(&read(row), p) {
                        write(&mut row.0, p, encoding);
                    }
                }
                None => {
                    index += 1;
                    let mut fields = Fields(vec![String::new(); width.max(16)]);
                    fields.0[0] = side.to_string();
                    fields.0[1] = p.number.to_string();
                    fields.0[2] = index.to_string();
                    // Unicode copies only where the file's rows have them.
                    for i in [17, 18, 19] {
                        if i < fields.0.len() {
                            fields.0[i] = encode_embedded("");
                        }
                    }
                    write(&mut fields, p, encoding);
                    rows.push(Row(fields));
                }
            }
        }
        if sorted {
            rows.sort_by_key(Row::number);
        }
    }
}

fn write(f: &mut Fields, p: &Player, encoding: dvw_codec::TextEncoding) {
    let len = f.0.len();
    let set = |f: &mut Fields, i: usize, v: String| {
        if i < f.0.len() {
            f.0[i] = v;
        }
    };
    // Plain text the file's code page can't hold becomes `?`; a Unicode
    // copy, when the row has one, keeps it.
    let plain = |s: &str| -> String {
        s.chars()
            .map(|c| match c {
                ';' => ',',
                c if c.is_control() => ' ',
                c if encoding.encode(&c.to_string()).is_none() => '?',
                c => c,
            })
            .collect()
    };
    set(f, 1, p.number.to_string());
    set(f, 8, p.code.clone());
    for (i, embedded, text) in [
        (9, 17, &p.last_name),
        (10, 18, &p.first_name),
        (11, 19, &p.nickname),
    ] {
        set(f, i, plain(text));
        if embedded < len && f.0[embedded].starts_with('\u{0F}') {
            f.0[embedded] = encode_embedded(text);
        }
    }
    let special = if p.libero {
        "L"
    } else if p.captain {
        "C"
    } else {
        ""
    };
    set(f, 12, special.to_owned());
    set(f, 13, p.role.map_or(String::new(), |r| r.to_string()));
    set(f, 14, if p.foreign { "True" } else { "False" }.to_owned());
    // DataVolley repeats the player id in column 15.
    if len > 15 {
        set(f, 15, p.code.clone());
    }
}
