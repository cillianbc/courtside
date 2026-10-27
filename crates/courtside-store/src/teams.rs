//! Teams, rosters and each match's own roster.
//!
//! A player has a stable id and a code unique in the season (written to
//! the `.dvw` player id column; analysis across matches goes by it). When
//! a match is created or imported, both rosters are copied into
//! `match_player`: the match is built and exported from that copy, so
//! editing a team's roster later never rewrites a match already coded.

use std::collections::HashSet;

pub use dvw_codec::team_file::{Person, Staff};
use rusqlite::{Connection, OptionalExtension, Transaction, params};

use crate::{Error, MatchId, Result, Season, next_id};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Team {
    pub id: String,
    pub name: String,
    /// Unique in the season when set; `[3TEAMS]` id column.
    pub code: String,
    /// Up to five characters: `.sq` abbreviation, `.vst` `shortCode`.
    pub short_code: String,
    pub staff: Staff,
    /// Home hall and city: a new match's venue when the team is at home.
    pub hall: String,
    pub city: String,
    /// Kit colours, `#rrggbb` or empty. The shirt colour is the `.dvw`
    /// `[3TEAMS]` colour.
    pub shirt_colour: String,
    pub libero_colour: String,
    /// The screen colour: a preset's key; empty for the default.
    pub colour: String,
    pub players: Vec<Player>,
    /// Matches the team plays in.
    pub matches: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Player {
    /// Empty for a match-roster row whose season player was deleted.
    pub id: String,
    pub number: u8,
    pub code: String,
    pub last_name: String,
    pub first_name: String,
    pub nickname: String,
    /// `.dvw` role: 1 libero, 2 outside, 3 opposite, 4 middle, 5 setter.
    pub role: Option<u8>,
    pub libero: bool,
    pub captain: bool,
    pub foreign: bool,
    /// ISO `YYYY-MM-DD`.
    pub birth_date: Option<String>,
    pub height_cm: Option<u16>,
    pub weight_kg: Option<u16>,
    /// Attack reach.
    pub reach_cm: Option<u16>,
}

/// A team's details as entered (see [`Team`]).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TeamInput {
    pub name: String,
    /// Empty: made from the name.
    pub code: String,
    pub short_code: String,
    pub staff: Staff,
    pub hall: String,
    pub city: String,
    pub shirt_colour: String,
    pub libero_colour: String,
    pub colour: String,
}

/// A roster row as entered. `id` is set for a player already on the
/// roster; `code` empty means "make one from the name".
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PlayerInput {
    pub id: Option<String>,
    pub number: u8,
    pub code: String,
    pub last_name: String,
    pub first_name: String,
    pub nickname: String,
    pub role: Option<u8>,
    pub libero: bool,
    pub captain: bool,
    pub foreign: bool,
    pub birth_date: Option<String>,
    pub height_cm: Option<u16>,
    pub weight_kg: Option<u16>,
    pub reach_cm: Option<u16>,
}

/// A player of the season with their team, for the players list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SeasonPlayer {
    pub player: Player,
    pub team_id: String,
    pub team_name: String,
    /// Matches the player is on the roster of.
    pub matches: usize,
}

const PLAYER_COLUMNS: &str = "id, number, code, last_name, first_name, nickname, role, libero, \
     captain, foreign_player, birth_date, height_cm, weight_kg, reach_cm";

/// `player_row`'s width.
const PLAYER_WIDTH: usize = 14;

fn player_row(r: &rusqlite::Row<'_>, at: usize) -> rusqlite::Result<Player> {
    Ok(Player {
        id: r.get::<_, Option<String>>(at)?.unwrap_or_default(),
        number: r.get(at + 1)?,
        code: r.get(at + 2)?,
        last_name: r.get(at + 3)?,
        first_name: r.get(at + 4)?,
        nickname: r.get(at + 5)?,
        role: r.get(at + 6)?,
        libero: r.get(at + 7)?,
        captain: r.get(at + 8)?,
        foreign: r.get(at + 9)?,
        birth_date: r.get(at + 10)?,
        height_cm: r.get(at + 11)?,
        weight_kg: r.get(at + 12)?,
        reach_cm: r.get(at + 13)?,
    })
}

/// Team columns in [`team_row`]'s order.
const TEAM_COLUMNS: &str = "t.id, t.name, t.code, t.short_code, t.coach_last, t.coach_first, \
     t.assistant_last, t.assistant_first, t.assistant2_last, t.assistant2_first, t.physio_last, \
     t.physio_first, t.doctor_last, t.doctor_first, t.hall, t.city, t.shirt_colour, \
     t.libero_colour, t.colour";

fn team_row(r: &rusqlite::Row<'_>) -> rusqlite::Result<Team> {
    let person = |at: usize| -> rusqlite::Result<Person> {
        Ok(Person {
            last_name: r.get(at)?,
            first_name: r.get(at + 1)?,
        })
    };
    Ok(Team {
        id: r.get(0)?,
        name: r.get(1)?,
        code: r.get(2)?,
        short_code: r.get(3)?,
        staff: Staff {
            coach: person(4)?,
            assistant: person(6)?,
            assistant2: person(8)?,
            physio: person(10)?,
            doctor: person(12)?,
        },
        hall: r.get(14)?,
        city: r.get(15)?,
        shirt_colour: r.get(16)?,
        libero_colour: r.get(17)?,
        colour: r.get(18)?,
        players: Vec::new(),
        matches: r.get::<_, i64>(19)?.try_into().unwrap_or(0),
    })
}

/// `#rrggbb` (any case) as given, else empty.
fn kit_colour(s: &str) -> Result<String> {
    let s = s.trim();
    if s.is_empty() {
        return Ok(String::new());
    }
    let hex = s.strip_prefix('#').unwrap_or(s);
    if hex.len() == 6 && hex.chars().all(|c| c.is_ascii_hexdigit()) {
        Ok(format!("#{}", hex.to_ascii_lowercase()))
    } else {
        Err(invalid(format!("{s} isn't a colour (#rrggbb)")))
    }
}

/// A `#rrggbb` colour as the Windows colour number `.dvw` files hold
/// (`0x00bbggrr`: red is 255).
pub fn colour_to_dvw(hex: &str) -> Option<u32> {
    let v = u32::from_str_radix(hex.strip_prefix('#')?, 16).ok()?;
    let (r, g, b) = (v >> 16, (v >> 8) & 0xff, v & 0xff);
    Some(r | (g << 8) | (b << 16))
}

/// A `.dvw` colour number as `#rrggbb`.
pub fn colour_from_dvw(n: u32) -> Option<String> {
    (n <= 0x00ff_ffff).then(|| {
        let (r, g, b) = (n & 0xff, (n >> 8) & 0xff, (n >> 16) & 0xff);
        format!("#{r:02x}{g:02x}{b:02x}")
    })
}

/// Where a match is played: `.dvw` `[3MORE]` arena and city.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Venue {
    pub hall: String,
    pub city: String,
}

impl Team {
    /// The team's home venue.
    pub fn venue(&self) -> Venue {
        Venue {
            hall: self.hall.clone(),
            city: self.city.clone(),
        }
    }
}

/// The code a player gets when none is entered: the first three letters
/// of the last and first names (`SMI-JOH`), then `-2`, `-3`… if taken.
pub fn suggest_player_code(last: &str, first: &str, number: u8, taken: &HashSet<String>) -> String {
    let part = |s: &str| -> String {
        s.chars()
            .filter(|c| c.is_alphanumeric())
            .take(3)
            .flat_map(char::to_uppercase)
            .collect()
    };
    let (l, f) = (part(last), part(first));
    let base = match (l.is_empty(), f.is_empty()) {
        (false, false) => format!("{l}-{f}"),
        (false, true) => l,
        (true, false) => f,
        (true, true) => format!("P{number}"),
    };
    unique(base, taken)
}

/// A team code made from its name: initials of up to three words, or the
/// first three letters of a one-word name, then a number if taken.
pub fn suggest_team_code(name: &str, taken: &HashSet<String>) -> String {
    let words: Vec<&str> = name.split_whitespace().collect();
    let base: String = if words.len() > 1 {
        words
            .iter()
            .filter_map(|w| w.chars().find(|c| c.is_alphanumeric()))
            .take(3)
            .flat_map(char::to_uppercase)
            .collect()
    } else {
        name.chars()
            .filter(|c| c.is_alphanumeric())
            .take(3)
            .flat_map(char::to_uppercase)
            .collect()
    };
    unique(if base.is_empty() { "T".into() } else { base }, taken)
}

fn unique(base: String, taken: &HashSet<String>) -> String {
    if !taken.contains(&base) {
        return base;
    }
    (2..)
        .map(|n| format!("{base}-{n}"))
        .find(|c| !taken.contains(c))
        .unwrap_or(base)
}

fn codes_in(conn: &Connection, sql: &str, except: &str) -> Result<HashSet<String>> {
    Ok(conn
        .prepare(sql)?
        .query_map([except], |r| r.get::<_, String>(0))?
        .collect::<rusqlite::Result<_>>()?)
}

fn invalid(msg: impl Into<String>) -> Error {
    Error::Invalid(msg.into())
}

impl Season {
    /// Every team with its roster and match count, by name.
    pub fn teams(&self) -> Result<Vec<Team>> {
        let mut teams: Vec<Team> = self
            .conn
            .prepare(&format!(
                "SELECT {TEAM_COLUMNS},
                        (SELECT COUNT(*) FROM match m
                         WHERE m.home_team_id = t.id OR m.visiting_team_id = t.id)
                 FROM team t ORDER BY t.name"
            ))?
            .query_map([], team_row)?
            .collect::<rusqlite::Result<_>>()?;
        let mut stmt = self.conn.prepare(&format!(
            "SELECT {PLAYER_COLUMNS} FROM player WHERE team_id = ?1 ORDER BY number"
        ))?;
        for t in &mut teams {
            t.players = stmt
                .query_map([&t.id], |r| player_row(r, 0))?
                .collect::<rusqlite::Result<_>>()?;
        }
        Ok(teams)
    }

    pub fn team(&self, id: &str) -> Result<Team> {
        self.teams()?
            .into_iter()
            .find(|t| t.id == id)
            .ok_or_else(|| invalid(format!("no team {id}")))
    }

    /// Adds a team with an empty roster; returns its id.
    pub fn create_team(&mut self, input: &TeamInput, now_ms: u64) -> Result<String> {
        let tx = self.conn.transaction()?;
        let id = team_create_in(&tx, input, now_ms)?;
        tx.commit()?;
        Ok(id)
    }

    pub fn update_team(&mut self, id: &str, input: &TeamInput) -> Result<()> {
        let tx = self.conn.transaction()?;
        team_update_in(&tx, id, input)?;
        tx.commit()?;
        Ok(())
    }

    /// Deletes a team and its roster. A team with matches can't be
    /// deleted (merge it into another instead).
    pub fn delete_team(&mut self, id: &str) -> Result<()> {
        let tx = self.conn.transaction()?;
        let matches: i64 = tx.query_row(
            "SELECT COUNT(*) FROM match WHERE home_team_id = ?1 OR visiting_team_id = ?1",
            [id],
            |r| r.get(0),
        )?;
        if matches > 0 {
            return Err(invalid(format!(
                "the team plays in {matches} match(es); merge it into another team instead"
            )));
        }
        tx.execute("DELETE FROM player WHERE team_id = ?1", [id])?;
        tx.execute("DELETE FROM team WHERE id = ?1", [id])?;
        tx.commit()?;
        Ok(())
    }

    /// Replaces a team's roster with `rows`, in one transaction: rows with
    /// an id update that player, rows without one add a player, players not
    /// in `rows` leave the roster (matches keep their own copy). Numbers
    /// must be unique in the team, codes in the season.
    pub fn save_roster(&mut self, team_id: &str, rows: &[PlayerInput], now_ms: u64) -> Result<()> {
        check_rows(rows)?;
        let tx = self.conn.transaction()?;
        roster_in(&tx, team_id, rows, now_ms)?;
        tx.commit()?;
        Ok(())
    }

    /// Creates (`id` `None`) or updates a team together with its roster,
    /// all or nothing; returns the team's id.
    pub fn save_team(
        &mut self,
        id: Option<&str>,
        input: &TeamInput,
        rows: &[PlayerInput],
        now_ms: u64,
    ) -> Result<String> {
        check_rows(rows)?;
        let tx = self.conn.transaction()?;
        let id = match id {
            Some(id) => {
                team_update_in(&tx, id, input)?;
                id.to_owned()
            }
            None => team_create_in(&tx, input, now_ms)?,
        };
        roster_in(&tx, &id, rows, now_ms)?;
        tx.commit()?;
        Ok(id)
    }

    /// Every player of the season, by last name.
    pub fn players(&self) -> Result<Vec<SeasonPlayer>> {
        let cols = PLAYER_COLUMNS
            .split(", ")
            .map(|c| format!("p.{c}"))
            .collect::<Vec<_>>()
            .join(", ");
        let mut stmt = self.conn.prepare(&format!(
            "SELECT {cols}, t.id, t.name,
                    (SELECT COUNT(*) FROM match_player mp WHERE mp.player_id = p.id)
             FROM player p JOIN team t ON t.id = p.team_id
             ORDER BY p.last_name, p.first_name, t.name"
        ))?;
        let rows = stmt.query_map([], |r| {
            Ok(SeasonPlayer {
                player: player_row(r, 0)?,
                team_id: r.get(PLAYER_WIDTH)?,
                team_name: r.get(PLAYER_WIDTH + 1)?,
                matches: r.get::<_, i64>(PLAYER_WIDTH + 2)?.try_into().unwrap_or(0),
            })
        })?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    /// Two records of one person: `drop`'s match-roster entries point at
    /// `keep` from now on, and `drop` is deleted.
    pub fn merge_players(&mut self, keep: &str, drop: &str) -> Result<()> {
        if keep == drop {
            return Err(invalid("pick two different players"));
        }
        let tx = self.conn.transaction()?;
        for id in [keep, drop] {
            let found: Option<String> = tx
                .query_row("SELECT id FROM player WHERE id = ?1", [id], |r| r.get(0))
                .optional()?;
            if found.is_none() {
                return Err(invalid(format!("no player {id}")));
            }
        }
        tx.execute(
            "UPDATE match_player SET player_id = ?1 WHERE player_id = ?2",
            [keep, drop],
        )?;
        tx.execute("DELETE FROM player WHERE id = ?1", [drop])?;
        tx.commit()?;
        Ok(())
    }

    /// Folds team `from` into `into` (the same club under another name):
    /// its matches become `into`'s, and each of its players either joins
    /// the `into` player paired with them in `pairs` or moves across.
    /// `from` is then deleted.
    pub fn merge_teams(
        &mut self,
        from: &str,
        into: &str,
        pairs: &[(String, String)],
    ) -> Result<()> {
        if from == into {
            return Err(invalid("pick two different teams"));
        }
        let tx = self.conn.transaction()?;
        let players: Vec<(String, u8)> = tx
            .prepare("SELECT id, number FROM player WHERE team_id = ?1")?
            .query_map([from], |r| Ok((r.get(0)?, r.get(1)?)))?
            .collect::<rusqlite::Result<_>>()?;
        for (id, number) in players {
            match pairs.iter().find(|(f, _)| *f == id) {
                Some((_, target)) => {
                    let ok: Option<String> = tx
                        .query_row(
                            "SELECT id FROM player WHERE id = ?1 AND team_id = ?2",
                            [target, into],
                            |r| r.get(0),
                        )
                        .optional()?;
                    if ok.is_none() {
                        return Err(invalid(format!(
                            "#{number}: pick a player of the other team"
                        )));
                    }
                    tx.execute(
                        "UPDATE match_player SET player_id = ?1 WHERE player_id = ?2",
                        [target, &id],
                    )?;
                    tx.execute("DELETE FROM player WHERE id = ?1", [&id])?;
                }
                None => {
                    let clash: Option<String> = tx
                        .query_row(
                            "SELECT id FROM player WHERE team_id = ?1 AND number = ?2",
                            params![into, number],
                            |r| r.get(0),
                        )
                        .optional()?;
                    if clash.is_some() {
                        return Err(invalid(format!(
                            "#{number} is taken in the other team: pair the two players or renumber one"
                        )));
                    }
                    tx.execute("UPDATE player SET team_id = ?1 WHERE id = ?2", [into, &id])?;
                }
            }
        }
        tx.execute(
            "UPDATE match SET home_team_id = ?1 WHERE home_team_id = ?2",
            [into, from],
        )?;
        tx.execute(
            "UPDATE match SET visiting_team_id = ?1 WHERE visiting_team_id = ?2",
            [into, from],
        )?;
        tx.execute("DELETE FROM team WHERE id = ?1", [from])?;
        tx.commit()?;
        Ok(())
    }

    /// The match's home and visiting teams with the match's own rosters.
    pub fn match_teams(&self, id: MatchId) -> Result<(Team, Team)> {
        let ids: Option<(String, String)> = self
            .conn
            .query_row(
                "SELECT home_team_id, visiting_team_id FROM match WHERE id = ?1",
                [id.to_string()],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()?;
        let (home_id, visiting_id) = ids.ok_or(Error::NoMatch(id))?;
        let mut home = self.team(&home_id)?;
        let mut visiting = self.team(&visiting_id)?;
        let (h, v) = self.match_roster(id)?;
        home.players = h;
        visiting.players = v;
        Ok((home, visiting))
    }

    /// The match's own rosters: home, visiting.
    pub fn match_roster(&self, id: MatchId) -> Result<(Vec<Player>, Vec<Player>)> {
        let mut stmt = self.conn.prepare(
            "SELECT player_id, number, code, last_name, first_name, nickname, role, libero,
                    captain, foreign_player, NULL, NULL, NULL, NULL
             FROM match_player WHERE match_id = ?1 AND side = ?2 ORDER BY number",
        )?;
        let mut side = |s: u8| -> Result<Vec<Player>> {
            Ok(stmt
                .query_map(params![id.to_string(), s], |r| player_row(r, 0))?
                .collect::<rusqlite::Result<_>>()?)
        };
        Ok((side(0)?, side(1)?))
    }

    /// Replaces one side's roster for this match only (a late addition, a
    /// corrected name). For an imported match the file's player rows follow
    /// on open and export.
    pub fn save_match_roster(
        &mut self,
        id: MatchId,
        home: bool,
        rows: &[PlayerInput],
    ) -> Result<()> {
        check_rows(rows)?;
        let tx = self.conn.transaction()?;
        let imported: Option<bool> = tx
            .query_row(
                "SELECT source_dvw IS NOT NULL FROM match WHERE id = ?1",
                [id.to_string()],
                |r| r.get(0),
            )
            .optional()?;
        if imported.is_none() {
            return Err(Error::NoMatch(id));
        }
        let side = u8::from(!home);
        tx.execute(
            "DELETE FROM match_player WHERE match_id = ?1 AND side = ?2",
            params![id.to_string(), side],
        )?;
        let mut taken = HashSet::new();
        for r in rows {
            let code = match r.code.trim() {
                "" => suggest_player_code(&r.last_name, &r.first_name, r.number, &taken),
                c => c.to_owned(),
            };
            taken.insert(code.clone());
            insert_match_player(&tx, id, side, r.id.as_deref(), r, &code)?;
        }
        tx.commit()?;
        Ok(())
    }
}

pub(crate) fn team_create_in(
    tx: &Transaction<'_>,
    input: &TeamInput,
    now_ms: u64,
) -> Result<String> {
    let id = next_id(tx, now_ms)?.to_string();
    // The row first, then every column as an update sets them.
    tx.execute(
        "INSERT INTO team (id, name, coach_last, assistant_last) VALUES (?1, ?2, '', '')",
        params![id, input.name.trim()],
    )
    .map_err(|e| unique_name(e, &input.name))?;
    team_update_in(tx, &id, input)?;
    Ok(id)
}

fn team_update_in(tx: &Transaction<'_>, id: &str, input: &TeamInput) -> Result<()> {
    let code = team_code(tx, input, id)?;
    let short_code: String = input.short_code.trim().chars().take(5).collect();
    let (shirt, libero) = (
        kit_colour(&input.shirt_colour)?,
        kit_colour(&input.libero_colour)?,
    );
    let st = &input.staff;
    let names = |p: &Person| {
        (
            p.last_name.trim().to_owned(),
            p.first_name.trim().to_owned(),
        )
    };
    let [c, a, a2, ph, d] = [
        &st.coach,
        &st.assistant,
        &st.assistant2,
        &st.physio,
        &st.doctor,
    ]
    .map(names);
    let n = tx
        .execute(
            "UPDATE team SET name = ?2, code = ?3, short_code = ?4, coach_last = ?5,
                 coach_first = ?6, assistant_last = ?7, assistant_first = ?8,
                 assistant2_last = ?9, assistant2_first = ?10, physio_last = ?11,
                 physio_first = ?12, doctor_last = ?13, doctor_first = ?14, hall = ?15,
                 city = ?16, shirt_colour = ?17, libero_colour = ?18, colour = ?19
             WHERE id = ?1",
            params![
                id,
                input.name.trim(),
                code,
                short_code,
                c.0,
                c.1,
                a.0,
                a.1,
                a2.0,
                a2.1,
                ph.0,
                ph.1,
                d.0,
                d.1,
                input.hall.trim(),
                input.city.trim(),
                shirt,
                libero,
                input.colour.trim()
            ],
        )
        .map_err(|e| unique_name(e, &input.name))?;
    if n == 0 {
        return Err(invalid(format!("no team {id}")));
    }
    Ok(())
}

fn roster_in(tx: &Transaction<'_>, team_id: &str, rows: &[PlayerInput], now_ms: u64) -> Result<()> {
    let mut taken = codes_in(tx, "SELECT code FROM player WHERE team_id <> ?1", team_id)?;
    let kept: Vec<&str> = rows.iter().filter_map(|r| r.id.as_deref()).collect();
    let current: Vec<String> = tx
        .prepare("SELECT id FROM player WHERE team_id = ?1")?
        .query_map([team_id], |r| r.get(0))?
        .collect::<rusqlite::Result<_>>()?;
    for id in current.iter().filter(|id| !kept.contains(&id.as_str())) {
        tx.execute("DELETE FROM player WHERE id = ?1", [id])?;
    }
    // Entered codes first, so a generated one never takes them.
    for r in rows {
        let code = r.code.trim();
        if !code.is_empty() && !taken.insert(code.to_owned()) {
            return Err(invalid(format!(
                "#{}: code {code} belongs to another player",
                r.number
            )));
        }
    }
    // Numbers move between rows (a swap), so park them first.
    tx.execute(
        "UPDATE player SET number = number + 1000 WHERE team_id = ?1",
        [team_id],
    )?;
    for r in rows {
        let code = match r.code.trim() {
            "" => {
                let c = suggest_player_code(&r.last_name, &r.first_name, r.number, &taken);
                taken.insert(c.clone());
                c
            }
            c => c.to_owned(),
        };
        let values = params![
            team_id,
            r.number,
            code,
            r.last_name.trim(),
            r.first_name.trim(),
            r.nickname.trim(),
            r.role,
            r.libero,
            r.captain,
            r.foreign,
            r.birth_date.as_deref().filter(|d| !d.is_empty()),
            r.height_cm,
            r.weight_kg,
            r.reach_cm,
        ];
        match &r.id {
            Some(id) => {
                let n = tx.execute(
                    "UPDATE player SET team_id = ?1, number = ?2, code = ?3, last_name = ?4,
                             first_name = ?5, nickname = ?6, role = ?7, libero = ?8, captain = ?9,
                             foreign_player = ?10, birth_date = ?11, height_cm = ?12,
                             weight_kg = ?13, reach_cm = ?14
                         WHERE id = ?15 AND team_id = ?1",
                    rusqlite::params_from_iter(
                        values.iter().copied().chain([id as &dyn rusqlite::ToSql]),
                    ),
                )?;
                if n == 0 {
                    return Err(invalid(format!("#{}: not a player of this team", r.number)));
                }
            }
            None => {
                let id = next_id(tx, now_ms)?.to_string();
                tx.execute(
                    "INSERT INTO player (team_id, number, code, last_name, first_name,
                                 nickname, role, libero, captain, foreign_player, birth_date,
                                 height_cm, weight_kg, reach_cm, id)
                             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13,
                                 ?14, ?15)",
                    rusqlite::params_from_iter(
                        values.iter().copied().chain([&id as &dyn rusqlite::ToSql]),
                    ),
                )?;
            }
        }
    }
    Ok(())
}

/// Numbers 0–99, unique; last names given; one row per id.
fn check_rows(rows: &[PlayerInput]) -> Result<()> {
    let mut numbers = HashSet::new();
    let mut codes = HashSet::new();
    let mut ids = HashSet::new();
    for r in rows {
        if r.number > 99 {
            return Err(invalid(format!(
                "#{}: shirt numbers go from 0 to 99",
                r.number
            )));
        }
        if !numbers.insert(r.number) {
            return Err(invalid(format!("#{} is on the roster twice", r.number)));
        }
        let code = r.code.trim();
        if !code.is_empty() && !codes.insert(code) {
            return Err(invalid(format!("code {code} is on the roster twice")));
        }
        if r.last_name.trim().is_empty() && r.nickname.trim().is_empty() {
            return Err(invalid(format!("#{}: enter a last name", r.number)));
        }
        if let Some(id) = &r.id
            && !ids.insert(id)
        {
            return Err(invalid(format!("#{}: the same player twice", r.number)));
        }
        if r.role.is_some_and(|x| !(1..=6).contains(&x)) {
            return Err(invalid(format!("#{}: unknown role", r.number)));
        }
    }
    Ok(())
}

fn team_code(tx: &Transaction<'_>, input: &TeamInput, id: &str) -> Result<String> {
    if input.name.trim().is_empty() {
        return Err(invalid("enter the team's name"));
    }
    let taken = codes_in(tx, "SELECT code FROM team WHERE id <> ?1", id)?;
    Ok(match input.code.trim() {
        "" => suggest_team_code(&input.name, &taken),
        c if taken.contains(c) => {
            return Err(invalid(format!("team code {c} is taken")));
        }
        c => c.to_owned(),
    })
}

fn unique_name(e: rusqlite::Error, name: &str) -> Error {
    match e {
        rusqlite::Error::SqliteFailure(f, _)
            if f.code == rusqlite::ErrorCode::ConstraintViolation =>
        {
            invalid(format!("there is already a team called {}", name.trim()))
        }
        e => e.into(),
    }
}

/// Copies both teams' current rosters into a new match.
pub(crate) fn copy_rosters(
    tx: &Transaction<'_>,
    id: MatchId,
    home: &str,
    visiting: &str,
) -> Result<()> {
    for (side, team) in [(0u8, home), (1, visiting)] {
        tx.execute(
            "INSERT INTO match_player (match_id, side, number, player_id, code, last_name,
                 first_name, nickname, role, libero, captain, foreign_player)
             SELECT ?1, ?2, number, id, code, last_name, first_name, nickname, role, libero,
                 captain, foreign_player
             FROM player WHERE team_id = ?3",
            params![id.to_string(), side, team],
        )?;
    }
    Ok(())
}

pub(crate) fn insert_match_player(
    tx: &Transaction<'_>,
    id: MatchId,
    side: u8,
    player_id: Option<&str>,
    r: &PlayerInput,
    code: &str,
) -> Result<()> {
    tx.execute(
        "INSERT INTO match_player (match_id, side, number, player_id, code, last_name,
             first_name, nickname, role, libero, captain, foreign_player)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
        params![
            id.to_string(),
            side,
            r.number,
            player_id,
            code,
            r.last_name.trim(),
            r.first_name.trim(),
            r.nickname.trim(),
            r.role,
            r.libero,
            r.captain,
            r.foreign
        ],
    )?;
    Ok(())
}

/// For an imported `.dvw` player: the season player it is (same code in
/// the team, else same shirt number), or a new one. Returns its id and
/// code, and whether it was added. Season data already there is left as
/// it is.
pub(crate) fn find_or_add_player(
    tx: &Transaction<'_>,
    team_id: &str,
    r: &PlayerInput,
    now_ms: u64,
) -> Result<(String, String, bool)> {
    let code = r.code.trim();
    let by_code: Option<(String, String)> = if code.is_empty() {
        None
    } else {
        tx.query_row(
            "SELECT id, code FROM player WHERE team_id = ?1 AND code = ?2",
            [team_id, code],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()?
    };
    let found = match by_code {
        Some(f) => Some(f),
        None => tx
            .query_row(
                "SELECT id, code FROM player WHERE team_id = ?1 AND number = ?2",
                params![team_id, r.number],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()?,
    };
    if let Some((id, code)) = found {
        return Ok((id, code, false));
    }
    let taken = codes_in(tx, "SELECT code FROM player WHERE id <> ?1", "")?;
    let code = if !code.is_empty() && !taken.contains(code) {
        code.to_owned()
    } else {
        suggest_player_code(&r.last_name, &r.first_name, r.number, &taken)
    };
    let id = next_id(tx, now_ms)?.to_string();
    tx.execute(
        "INSERT INTO player (id, team_id, number, code, last_name, first_name, nickname, role,
             libero, captain, foreign_player)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
        params![
            id,
            team_id,
            r.number,
            code,
            r.last_name.trim(),
            r.first_name.trim(),
            r.nickname.trim(),
            r.role,
            r.libero,
            r.captain,
            r.foreign
        ],
    )?;
    if r.birth_date.is_some()
        || r.height_cm.is_some()
        || r.weight_kg.is_some()
        || r.reach_cm.is_some()
    {
        tx.execute(
            "UPDATE player SET birth_date = ?2, height_cm = ?3, weight_kg = ?4, reach_cm = ?5
             WHERE id = ?1",
            params![id, r.birth_date, r.height_cm, r.weight_kg, r.reach_cm],
        )?;
    }
    Ok((id, code, true))
}

/// What importing a team file did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TeamImport {
    pub team_id: String,
    pub team_name: String,
    /// The team wasn't in the season yet.
    pub created: bool,
    pub players_added: usize,
    /// Players already on the roster (by code or shirt number), left as
    /// they are.
    pub players_kept: usize,
}

impl Season {
    /// The team as a team file: `.vst` JSON when `vst`, else DataVolley's
    /// `.sq` layout (which VolleyStation reads too).
    pub fn export_team_file(&self, team_id: &str, vst: bool) -> Result<Vec<u8>> {
        use dvw_codec::team_file::{TeamFile, TeamFilePlayer, write_sq, write_vst};
        let t = self.team(team_id)?;
        let file = TeamFile {
            short_code: if t.short_code.is_empty() {
                t.code.chars().take(5).collect()
            } else {
                t.short_code
            },
            code: t.code,
            name: t.name,
            staff: t.staff,
            home_court: t.hall,
            players: t
                .players
                .into_iter()
                .map(|p| TeamFilePlayer {
                    number: p.number,
                    code: p.code,
                    last_name: p.last_name,
                    first_name: p.first_name,
                    nickname: p.nickname,
                    role: p.role,
                    libero: p.libero,
                    captain: p.captain,
                    foreign: p.foreign,
                    birth_date: p.birth_date,
                    height_cm: p.height_cm,
                    weight_kg: p.weight_kg,
                    reach_cm: p.reach_cm,
                })
                .collect(),
        };
        Ok(if vst {
            write_vst(&file)
        } else {
            write_sq(&file)
        })
    }

    /// Imports a team file (`.sq` or `.vst`): a team already in the season
    /// (same code, else same name) is reused and gains the players it
    /// lacks; players it has, by code or shirt number, are left as they
    /// are.
    pub fn import_team_file(
        &mut self,
        file_name: &str,
        bytes: &[u8],
        now_ms: u64,
    ) -> Result<TeamImport> {
        let is_json = file_name.to_ascii_lowercase().ends_with(".vst")
            || bytes.iter().find(|b| !b.is_ascii_whitespace()) == Some(&b'{');
        let file = if is_json {
            dvw_codec::team_file::parse_vst(bytes)
        } else {
            dvw_codec::team_file::parse_sq(bytes)
        }
        .map_err(|e| invalid(e.to_string()))?;
        let name = if file.name.is_empty() {
            file.code.clone()
        } else {
            file.name.clone()
        };
        let tx = self.conn.transaction()?;
        let existing: Option<String> = match file.code.as_str() {
            "" => None,
            code => tx
                .query_row("SELECT id FROM team WHERE code = ?1", [code], |r| r.get(0))
                .optional()?,
        };
        let existing = match existing {
            Some(id) => Some(id),
            None => tx
                .query_row("SELECT id FROM team WHERE name = ?1", [&name], |r| r.get(0))
                .optional()?,
        };
        let created = existing.is_none();
        let team_id = match existing {
            Some(id) => id,
            None => {
                let taken = codes_in(&tx, "SELECT code FROM team WHERE id <> ?1", "")?;
                let code = match file.code.as_str() {
                    c if !c.is_empty() && !taken.contains(c) => c.to_owned(),
                    _ => String::new(),
                };
                team_create_in(
                    &tx,
                    &TeamInput {
                        name: name.clone(),
                        code,
                        short_code: file.short_code.clone(),
                        staff: file.staff.clone(),
                        hall: file.home_court.clone(),
                        ..TeamInput::default()
                    },
                    now_ms,
                )?
            }
        };
        let (mut added, mut kept) = (0, 0);
        for p in &file.players {
            let row = PlayerInput {
                id: None,
                number: p.number,
                code: p.code.clone(),
                last_name: p.last_name.clone(),
                first_name: p.first_name.clone(),
                nickname: p.nickname.clone(),
                role: p.role,
                libero: p.libero,
                captain: p.captain,
                foreign: p.foreign,
                birth_date: p.birth_date.clone(),
                height_cm: p.height_cm,
                weight_kg: p.weight_kg,
                reach_cm: p.reach_cm,
            };
            if find_or_add_player(&tx, &team_id, &row, now_ms)?.2 {
                added += 1;
            } else {
                kept += 1;
            }
        }
        tx.commit()?;
        Ok(TeamImport {
            team_id,
            team_name: name,
            created,
            players_added: added,
            players_kept: kept,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn player_codes() {
        let mut taken = HashSet::new();
        assert_eq!(suggest_player_code("Smith", "John", 4, &taken), "SMI-JOH");
        taken.insert("SMI-JOH".to_owned());
        assert_eq!(
            suggest_player_code("Smith", "Johanna", 5, &taken),
            "SMI-JOH-2"
        );
        assert_eq!(suggest_player_code("Ó Sé", "", 7, &taken), "ÓSÉ");
        assert_eq!(suggest_player_code("", "", 12, &taken), "P12");
    }

    #[test]
    fn kit_colours_as_dvw_numbers() {
        assert_eq!(colour_to_dvw("#ff0000"), Some(255));
        assert_eq!(colour_to_dvw("#0b63f4"), Some(16_016_139));
        assert_eq!(colour_from_dvw(32768).as_deref(), Some("#008000"));
        assert_eq!(colour_from_dvw(16_777_215).as_deref(), Some("#ffffff"));
        assert_eq!(kit_colour(" #ABCDEF ").ok().as_deref(), Some("#abcdef"));
        assert!(kit_colour("red").is_err());
    }

    #[test]
    fn team_codes() {
        let taken: HashSet<String> = ["HCV".to_owned()].into();
        assert_eq!(suggest_team_code("Northgate", &taken), "NOR");
        assert_eq!(suggest_team_code("Harbour City VC", &taken), "HCV-2");
    }
}
