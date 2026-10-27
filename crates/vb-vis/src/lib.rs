//! The FIVB VIS web service's public requests (no login): tournaments, a
//! tournament's matches, and a team's registered players, so an
//! international match can be set up from VIS instead of typed in. Requests
//! and fields are as the FIVB documents them
//! (<https://www.fivb.org/VisSDK/VisWebService/>); openvolley's `fivbvis`
//! (R, MIT) does the same reading. Only the request text and the response
//! reading are here: the caller sends a request (`POST` to [`ENDPOINT`]
//! with `Accept: application/json`) and hands back the body.
//!
//! Uploading live data (`UploadVolleyLive`) needs the scout's FIVB
//! credentials and isn't done.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use vb_scoresheet::{Scoresheet, SheetPlayer, SheetTeam};

pub const ENDPOINT: &str = "https://www.fivb.org/Vis2009/XmlRequest.asmx";

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum Error {
    #[error("VIS sent something unexpected: {0}")]
    Response(String),
    #[error("VIS says: {0}")]
    Vis(String),
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Tournament {
    pub no: u32,
    pub code: String,
    pub name: String,
    pub season: String,
    /// ISO `YYYY-MM-DD`.
    pub start_date: Option<String>,
    pub end_date: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VisTeam {
    /// The team's registration in the tournament.
    pub no: u32,
    pub code: String,
    pub name: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VisMatch {
    pub no: u32,
    /// The match number within its tournament.
    pub no_in_tournament: Option<u32>,
    /// Local date and time, `YYYY-MM-DDTHH:MM:SS`.
    pub date_time_local: String,
    pub city: String,
    pub hall: String,
    /// Team A first, as VIS lists them (the home team).
    pub teams: [VisTeam; 2],
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VisPlayer {
    /// The FIVB's number for the player.
    pub no_player: u32,
    pub shirt: Option<u8>,
    pub last_name: String,
    pub first_name: String,
    pub captain: bool,
    pub libero: bool,
    pub height_cm: Option<u16>,
    pub weight_kg: Option<u16>,
    pub spike_cm: Option<u16>,
    pub birth_date: Option<String>,
}

/// XML text can't hold a bare `"`, `<` or `&` in an attribute.
fn attr(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('"', "&quot;")
}

pub fn tournaments_request() -> String {
    r#"<Request Type="GetVolleyTournamentList" Fields="No Code Name Season StartDate EndDate"/>"#
        .into()
}

pub fn matches_request(no_tournament: u32) -> String {
    format!(
        r#"<Request Type="GetVolleyMatchList" Fields="No NoInTournament DateTimeLocal City Hall NoTeamA NoTeamB"><Filter NoTournament="{}"/><Relation Name="TeamA" Fields="Code Name"/><Relation Name="TeamB" Fields="Code Name"/></Request>"#,
        attr(&no_tournament.to_string())
    )
}

pub fn players_request(no_team: u32) -> String {
    format!(
        r#"<Request Type="GetVolleyPlayerList" Fields="NoPlayer NoShirt IsCaptain IsLibero Height Weight Spike"><Filter NoTeam="{}"/><Relation Name="Player" Fields="FirstName LastName BirthDate"/></Request>"#,
        attr(&no_team.to_string())
    )
}

/// The `data` of a response, or VIS's own error.
fn data(body: &str) -> Result<Value, Error> {
    let v: Value = serde_json::from_str(body).map_err(|e| Error::Response(e.to_string()))?;
    if let Some(d) = v.get("data") {
        return Ok(d.clone());
    }
    let message = v
        .get("errors")
        .or_else(|| v.get("error"))
        .map_or_else(|| body.chars().take(200).collect(), ToString::to_string);
    Err(Error::Vis(message))
}

fn text(v: &Value, k: &str) -> String {
    match v.get(k) {
        Some(Value::String(s)) => s.trim().to_owned(),
        Some(Value::Number(n)) => n.to_string(),
        _ => String::new(),
    }
}

fn int(v: &Value, k: &str) -> Option<u64> {
    match v.get(k)? {
        Value::Number(n) => n.as_u64(),
        Value::String(s) => s.trim().parse().ok(),
        _ => None,
    }
}

/// VIS writes yes and no as `1` and `0` (or `true`/`false`).
fn flag(v: &Value, k: &str) -> bool {
    match v.get(k) {
        Some(Value::Bool(b)) => *b,
        Some(other) => {
            int(
                &Value::Object([(k.to_owned(), other.clone())].into_iter().collect()),
                k,
            ) == Some(1)
        }
        None => false,
    }
}

/// A length in VIS's unit (a millionth of a metre: 1 840 000 is 1.84 m) in
/// centimetres, when it's a person's.
fn cm(v: &Value, k: &str) -> Option<u16> {
    let n = int(v, k)? / 10_000;
    u16::try_from(n).ok().filter(|c| (100..=450).contains(c))
}

/// A weight in VIS's unit (a millionth of a kilogram) in kilograms.
fn kg(v: &Value, k: &str) -> Option<u16> {
    let n = int(v, k)? / 1_000_000;
    u16::try_from(n).ok().filter(|w| (30..=200).contains(w))
}

fn date(v: &Value, k: &str) -> Option<String> {
    let s = text(v, k);
    (s.len() >= 10 && s.as_bytes()[4] == b'-').then(|| s[..10].to_owned())
}

pub fn parse_tournaments(body: &str) -> Result<Vec<Tournament>, Error> {
    let d = data(body)?;
    let list = d
        .as_array()
        .ok_or_else(|| Error::Response("no list of tournaments".into()))?;
    Ok(list
        .iter()
        .filter_map(|t| {
            Some(Tournament {
                no: u32::try_from(int(t, "no")?).ok()?,
                code: text(t, "code"),
                name: text(t, "name"),
                season: text(t, "season"),
                start_date: date(t, "startDate"),
                end_date: date(t, "endDate"),
            })
        })
        .collect())
}

pub fn parse_matches(body: &str) -> Result<Vec<VisMatch>, Error> {
    let d = data(body)?;
    let list = d
        .as_array()
        .ok_or_else(|| Error::Response("no list of matches".into()))?;
    let mut out: Vec<VisMatch> = list
        .iter()
        .filter_map(|m| {
            let team = |no: &str, rel: &str| -> Option<VisTeam> {
                let t = m.get(rel).cloned().unwrap_or(Value::Null);
                Some(VisTeam {
                    no: u32::try_from(int(m, no)?).ok()?,
                    code: text(&t, "code"),
                    name: text(&t, "name"),
                })
            };
            Some(VisMatch {
                no: u32::try_from(int(m, "no")?).ok()?,
                no_in_tournament: int(m, "noInTournament").and_then(|n| u32::try_from(n).ok()),
                date_time_local: text(m, "dateTimeLocal"),
                city: text(m, "city"),
                hall: text(m, "hall"),
                teams: [team("noTeamA", "teamA")?, team("noTeamB", "teamB")?],
            })
        })
        .collect();
    out.sort_by(|a, b| {
        a.date_time_local
            .cmp(&b.date_time_local)
            .then(a.no.cmp(&b.no))
    });
    Ok(out)
}

pub fn parse_players(body: &str) -> Result<Vec<VisPlayer>, Error> {
    let d = data(body)?;
    let list = d
        .as_array()
        .ok_or_else(|| Error::Response("no list of players".into()))?;
    let mut out: Vec<VisPlayer> = list
        .iter()
        .filter_map(|p| {
            let person = p.get("player").cloned().unwrap_or(Value::Null);
            Some(VisPlayer {
                no_player: u32::try_from(int(p, "noPlayer")?).ok()?,
                shirt: int(p, "noShirt")
                    .and_then(|n| u8::try_from(n).ok())
                    .filter(|n| *n <= 99),
                last_name: text(&person, "lastName"),
                first_name: text(&person, "firstName"),
                captain: flag(p, "isCaptain"),
                libero: flag(p, "isLibero"),
                height_cm: cm(p, "height"),
                weight_kg: kg(p, "weight"),
                spike_cm: cm(p, "spike"),
                birth_date: date(&person, "birthdate").or_else(|| date(&person, "birthDate")),
            })
        })
        .collect();
    out.sort_by_key(|p| (p.shirt.is_none(), p.shirt, p.no_player));
    Ok(out)
}

/// A VIS match as a scoresheet with no sets yet: its teams with their
/// registered players (those with a shirt number), date, time, venue and
/// match number, team A as home. A player's code is the FIVB's number.
pub fn sheet(tournament: &Tournament, m: &VisMatch, players: [&[VisPlayer]; 2]) -> Scoresheet {
    let (date, time) = match m.date_time_local.split_once('T') {
        Some((d, t)) => (
            Some(d.to_owned()).filter(|d| d.len() == 10),
            t.get(..5).unwrap_or("").to_owned(),
        ),
        None => (None, String::new()),
    };
    let team = |i: usize| SheetTeam {
        name: m.teams[i].name.clone(),
        letter: ["A", "B"][i].into(),
        players: players[i]
            .iter()
            .filter_map(|p| {
                Some(SheetPlayer {
                    number: p.shirt?,
                    last_name: p.last_name.clone(),
                    first_name: p.first_name.clone(),
                    id: p.no_player.to_string(),
                    libero: p.libero,
                    captain: p.captain,
                    height_cm: p.height_cm,
                    weight_kg: p.weight_kg,
                    reach_cm: p.spike_cm,
                    birth_date: p.birth_date.clone(),
                })
            })
            .collect(),
        staff: Vec::new(),
    };
    Scoresheet {
        source: format!("FIVB VIS match {}", m.no),
        competition: tournament.name.clone(),
        match_number: m
            .no_in_tournament
            .map_or_else(String::new, |n| n.to_string()),
        date,
        time,
        city: m.city.clone(),
        hall: m.hall.clone(),
        teams: [team(0), team(1)],
        ..Scoresheet::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Made-up responses in the shapes VIS sends (2026-10-27).
    const TOURNAMENTS: &str = r#"{"data":[{"no":10,"code":"MHL2027","name":"Harbour League 2027","season":"2027","startDate":"2027-01-10","endDate":null},{"no":11,"code":"X","name":"Other","season":2026,"startDate":null}]}"#;
    const MATCHES: &str = r#"{"data":[{"no":502,"noInTournament":2,"dateTimeLocal":"2027-03-15T19:00:00","city":"Northgate","hall":"North Hall","noTeamA":71,"noTeamB":70,"teamA":{"code":"NGV","name":"Northgate"},"teamB":{"code":"HCV","name":"Harbour City"}},{"no":501,"noInTournament":1,"dateTimeLocal":"2027-03-14T19:45:00","city":"Harbourtown","hall":"Quayside Arena","noTeamA":70,"noTeamB":71,"teamA":{"code":"HCV","name":"Harbour City"},"teamB":{"code":"NGV","name":"Northgate"}}],"nbItems":2,"version":7}"#;
    const PLAYERS: &str = r#"{"data":[{"noPlayer":900012,"noShirt":12,"isCaptain":1,"isLibero":0,"height":1920000,"weight":84000000,"spike":3380000,"player":{"firstName":"Jonah","lastName":"Ardent","birthdate":"2001-03-14"}},{"noPlayer":900007,"noShirt":7,"isCaptain":0,"isLibero":1,"height":null,"player":{"firstName":"Maya","lastName":"Quill"}},{"noPlayer":900099,"noShirt":null,"isCaptain":0,"isLibero":0,"player":{"firstName":"Reserve","lastName":"Nobody"}}]}"#;

    #[test]
    fn reads_tournaments_matches_and_players() {
        let t = parse_tournaments(TOURNAMENTS).expect("tournaments");
        assert_eq!(t.len(), 2);
        assert_eq!(
            (t[0].no, t[0].code.as_str(), t[0].start_date.as_deref()),
            (10, "MHL2027", Some("2027-01-10"))
        );
        assert_eq!(t[1].season, "2026", "a number reads as text");

        let m = parse_matches(MATCHES).expect("matches");
        assert_eq!(m[0].no, 501, "by date");
        assert_eq!(
            (m[0].teams[0].no, m[0].teams[0].name.as_str()),
            (70, "Harbour City")
        );
        assert_eq!(m[0].hall, "Quayside Arena");

        let p = parse_players(PLAYERS).expect("players");
        assert_eq!(
            p.iter().map(|p| p.shirt).collect::<Vec<_>>(),
            [Some(7), Some(12), None]
        );
        let a = &p[1];
        assert!(a.captain && !a.libero);
        assert_eq!(
            (a.height_cm, a.weight_kg, a.spike_cm),
            (Some(192), Some(84), Some(338))
        );
        assert_eq!(a.birth_date.as_deref(), Some("2001-03-14"));
        assert!(p[0].libero);
        assert_eq!(p[0].height_cm, None);
    }

    #[test]
    fn a_match_becomes_a_scoresheet_without_sets() {
        let t = &parse_tournaments(TOURNAMENTS).expect("t")[0];
        let m = &parse_matches(MATCHES).expect("m")[0];
        let p = parse_players(PLAYERS).expect("p");
        let s = sheet(t, m, [&p, &[]]);
        assert_eq!(s.source, "FIVB VIS match 501");
        assert_eq!(
            (s.date.as_deref(), s.time.as_str()),
            (Some("2027-03-14"), "19:45")
        );
        assert_eq!(
            (s.competition.as_str(), s.match_number.as_str()),
            ("Harbour League 2027", "1")
        );
        assert_eq!(s.teams[0].name, "Harbour City");
        assert_eq!(
            s.teams[0].players.len(),
            2,
            "only players with a shirt number"
        );
        assert_eq!(s.teams[0].players[1].id, "900012");
        assert_eq!(s.teams[0].players[1].reach_cm, Some(338));
        assert!(s.sets.is_empty());
    }

    #[test]
    fn requests_and_errors() {
        assert!(matches_request(979).contains(r#"<Filter NoTournament="979"/>"#));
        assert!(players_request(3004).contains(r#"<Filter NoTeam="3004"/>"#));
        assert!(matches!(
            parse_matches(r#"{"errors":[{"code":"AccessDenied"}]}"#),
            Err(Error::Vis(_))
        ));
        assert!(matches!(parse_matches("<html>"), Err(Error::Response(_))));
    }
}
