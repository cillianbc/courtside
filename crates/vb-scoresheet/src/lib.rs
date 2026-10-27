//! Official volleyball scoresheets (issue #57): what one records about a
//! match, read from the PDF a scoring program prints. Only eScoresheet's
//! layout (the FIVB scoresheet) for now; the samples it was built from are
//! local only, like the match corpus.
//!
//! Teams are `[left, right]` as the sheet's title names them: the home team
//! first (left), the visitors second.

pub mod escoresheet;
pub mod pdf;

use serde::{Deserialize, Serialize};

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum Error {
    #[error("can't read the PDF: {0}")]
    Pdf(String),
    #[error("the PDF has no pages")]
    Empty,
    #[error("not a scoresheet Courtside can read: {0}")]
    NotScoresheet(String),
}

/// Reads a scoresheet PDF.
pub fn read_pdf(bytes: &[u8]) -> Result<Scoresheet, Error> {
    escoresheet::parse(&pdf::first_page(bytes)?)
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Scoresheet {
    /// The program that printed it, with its release (`eScoresheet 2026.10.04`).
    pub source: String,
    pub competition: String,
    pub category: String,
    pub match_number: String,
    /// The round or matchday.
    pub day: String,
    /// ISO `YYYY-MM-DD`.
    pub date: Option<String>,
    /// `HH:MM`.
    pub time: String,
    pub city: String,
    pub hall: String,
    pub country: String,
    /// Home (left in the title), then visitors.
    pub teams: [SheetTeam; 2],
    pub sets: Vec<SheetSet>,
    /// First and second referee.
    pub referees: Vec<String>,
    pub scorer: String,
    pub assistant_scorer: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SheetTeam {
    pub name: String,
    /// `A` or `B`, from the toss.
    pub letter: String,
    pub players: Vec<SheetPlayer>,
    /// Officials on the bench (coach `C`, assistant `AC`, …).
    pub staff: Vec<SheetStaff>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SheetPlayer {
    pub number: u8,
    pub last_name: String,
    pub first_name: String,
    /// The licence number in brackets after the name, if given.
    pub id: String,
    pub libero: bool,
    pub captain: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SheetStaff {
    /// As the sheet writes it: `C` coach, `AC` assistant coach, …
    pub role: String,
    pub last_name: String,
    pub first_name: String,
    pub id: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SheetSet {
    pub number: u8,
    /// `HH:MM`, as printed.
    pub start: String,
    pub end: String,
    /// Index into `teams` of the team that served first.
    pub served_first: Option<usize>,
    /// Each team's starting six in rotation order I–VI (zone 1 = I, the
    /// server), by team index.
    pub lineups: [Vec<u8>; 2],
    pub substitutions: Vec<Substitution>,
    /// Points, by team index, from the results table.
    pub score: Option<[u8; 2]>,
    pub minutes: Option<u16>,
}

/// A substitution as the sheet records it: in the column of the starting
/// player it replaced.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Substitution {
    pub team: usize,
    /// Starting position I–VI as 1–6.
    pub position: u8,
    /// The starter of that position.
    pub player_out: u8,
    pub player_in: u8,
    /// The score when the substitute came on, the substituting team's own
    /// score first (checked against a coded match: the sheet's `11:14` for
    /// the visitors is 14:11 home first).
    pub score_in: Option<[u8; 2]>,
    /// When the starter came back, if they did.
    pub score_back: Option<[u8; 2]>,
}
