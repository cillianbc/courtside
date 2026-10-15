//! The `.dvw` a match coded in the console starts from: the 14 sections
//! every corpus file has (FORMAT-NOTES "Sections"), headers filled from the
//! season's tables, an empty `[3SCOUT]`. The engine fills in everything it
//! derives (sets, starting zones, scout lines) on export, as for imports.

use dvw_codec::date::Date;

use crate::{Tables, Team};

/// The template for a match between `home` and `visiting` on `date`
/// (ISO `YYYY-MM-DD`, if known), as `.dvw` bytes (code page 1252, CRLF).
/// `tables`: the winning symbols, combinations and setter calls the match
/// was created with. `created_ms`: when the match was created (Unix ms),
/// written as the file's creation and last-change time.
pub fn blank(
    home: &Team,
    visiting: &Team,
    date: Option<&str>,
    tables: &Tables,
    created_ms: u64,
) -> Vec<u8> {
    let date = date.and_then(parse_iso);
    let written = date.map_or(String::new(), |d| {
        format!("{:02}/{:02}/{:04}", d.month, d.day, d.year)
    });
    // Spreadsheet serial, day 0 = 1899-12-30: the date DataVolley trusts.
    let serial = date.map_or(String::new(), |d| (d.to_unix_days() + 25_569).to_string());

    let stamp = stamp(created_ms);

    // DataVolley and VolleyStation both write the creation and last-change
    // blocks; a file without them is one no other program has made.
    let mut lines: Vec<String> = vec!["[3DATAVOLLEYSCOUT]".into(), "FILEFORMAT: 2.0".into()];
    for block in ["GENERATOR", "LASTCHANGE"] {
        lines.extend([
            format!("{block}-DAY: {stamp}"),
            format!("{block}-IDP: DVW"),
            format!("{block}-PRG: Courtside"),
            format!("{block}-REL: {}", env!("CARGO_PKG_VERSION")),
            format!("{block}-VER: "),
            format!("{block}-NAM: "),
        ]);
    }
    lines.extend([
        "[3MATCH]".into(),
        // date; time; season; league; phase; home/away; day; match number;
        // code page; regulation (1 = indoor rally point); zones; 0
        format!("{written};;;;;;;;1252;1;Z;0;"),
        format!(";;{serial};;;;;;;;"),
        "[3TEAMS]".into(),
        team_line(1, home),
        team_line(2, visiting),
        "[3MORE]".into(),
        ";;;;;;".into(),
        "[3COMMENTS]".into(),
        "[3SET]".into(),
    ]);
    lines.extend(std::iter::repeat_n("True;;;;;;".to_owned(), 5));
    for (side, header, team) in [(0, "[3PLAYERS-H]", home), (1, "[3PLAYERS-V]", visiting)] {
        lines.push(header.into());
        for (i, p) in team.players.iter().enumerate() {
            let id = field(&p.code);
            let special = if p.libero {
                "L"
            } else if p.captain {
                "C"
            } else {
                ""
            };
            let role = p
                .role
                .or(p.libero.then_some(1))
                .map_or(String::new(), |r| r.to_string());
            // team; number; index; zones 1–5; id; last; first; nickname;
            // special role (L libero, C captain); role; foreign; id again
            lines.push(format!(
                "{side};{};{};;;;;;{id};{};{};{};{special};{role};{};{id};;",
                p.number,
                i + 1,
                field(&p.last_name),
                field(&p.first_name),
                field(&p.nickname),
                if p.foreign { "True" } else { "False" },
            ));
        }
    }
    lines.push("[3ATTACKCOMBINATION]".into());
    lines.extend(tables.combination_lines());
    lines.push("[3SETTERCALL]".into());
    lines.extend(tables.setter_call_lines());
    lines.push("[3WINNINGSYMBOLS]".into());
    lines.push(tables.winning_line());
    lines.extend(["[3RESERVE]", "[3VIDEO]", "[3SCOUT]"].map(str::to_owned));

    let mut text = lines.join("\r\n");
    text.push_str("\r\n");
    let (bytes, _, _) = encoding_rs::WINDOWS_1252.encode(&text);
    bytes.into_owned()
}

/// `MM/DD/YYYY HH.MM.SS` in UTC, the order and time style of the corpus's
/// DataVolley files (FORMAT-NOTES "Header").
fn stamp(ms: u64) -> String {
    const DAY_MS: u64 = 86_400_000;
    let days = i64::try_from(ms / DAY_MS).unwrap_or(0);
    let secs = (ms % DAY_MS) / 1000;
    let date = Date::from_serial(days + 25_569).map_or(String::new(), |d| {
        format!("{:02}/{:02}/{:04}", d.month, d.day, d.year)
    });
    format!(
        "{date} {:02}.{:02}.{:02}",
        secs / 3600,
        secs / 60 % 60,
        secs % 60
    )
}

fn team_line(n: u8, t: &Team) -> String {
    // id; name; sets won; coach; assistant; shirt colour
    let id = if t.code.is_empty() {
        n.to_string()
    } else {
        field(&t.code)
    };
    format!(
        "{id};{};0;{};{};;",
        field(&t.name),
        field(&t.coach),
        field(&t.assistant)
    )
}

/// Text that can't break the positional format: no separators or newlines.
fn field(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            ';' => ',',
            '\r' | '\n' => ' ',
            c => c,
        })
        .collect()
}

fn parse_iso(s: &str) -> Option<Date> {
    let mut parts = s.splitn(3, '-');
    let year = parts.next()?.parse().ok()?;
    let month = parts.next()?.parse().ok()?;
    let day = parts.next()?.parse().ok()?;
    Date::new(year, month, day)
}
