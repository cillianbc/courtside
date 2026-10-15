//! Season settings (PRD F-03, F-17): typing defaults, winning symbols,
//! attack combinations, setter calls and the keymap, kept as one JSON
//! document in the season file. A new match freezes a copy of the tables
//! the `.dvw` carries (winning symbols, combinations, setter calls), so
//! changing the settings later never rewrites a match already coded.

use rusqlite::OptionalExtension;
use serde::{Deserialize, Serialize};

use crate::{MatchId, Result, Season};

/// Skill letters in the order the settings list them.
pub const SKILLS: [char; 7] = ['S', 'R', 'E', 'A', 'B', 'D', 'F'];

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    pub typing: Typing,
    pub tables: Tables,
    pub keymap: Vec<KeyBinding>,
    pub rules: Rules,
}

/// The competition's rule options and how the console ends a set. They
/// apply to every match of the season as it is opened.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Rules {
    /// Substitutions per team and set.
    pub substitutions: u8,
    /// A libero may serve.
    pub libero_may_serve: bool,
    /// Hold each set end the console would write until the scout confirms.
    pub confirm_set_end: bool,
}

impl Default for Rules {
    fn default() -> Self {
        Rules {
            substitutions: 6,
            libero_may_serve: false,
            confirm_set_end: false,
        }
    }
}

/// What the command line fills in (see `vb_grammar::expand`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Typing {
    /// Skill letter when none is typed and none is predicted.
    pub skill: char,
    /// Type per skill, in [`SKILLS`] order.
    pub types: [char; 7],
    /// Evaluation per skill, in [`SKILLS`] order.
    pub evaluations: [char; 7],
    /// Predict the skill from the touch before.
    pub predict: bool,
}

impl Default for Typing {
    fn default() -> Self {
        Typing {
            skill: 'A',
            types: ['H'; 7],
            evaluations: ['+'; 7],
            predict: true,
        }
    }
}

/// The tables a `.dvw` carries; frozen into each match when it is created.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Tables {
    /// One per skill in [`SKILLS`] order.
    pub winning: Vec<WinningRow>,
    pub combinations: Vec<Combination>,
    pub setter_calls: Vec<SetterCall>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WinningRow {
    pub skill: char,
    /// Evaluations that lose the rally for the touching team.
    pub lose: String,
    /// Evaluations that win it.
    pub win: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Combination {
    /// Two characters, e.g. `X5`.
    pub code: String,
    /// Zone the attack starts from, 1–9.
    pub start_zone: u8,
    /// The attack's type letter (`Q`, `T`, `H` …).
    pub tempo: char,
    /// Who the set goes to: `F` front, `B` back, `C` centre, `P` pipe,
    /// `S` setter, or empty.
    #[serde(default)]
    pub target: String,
    #[serde(default)]
    pub description: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetterCall {
    /// Two characters starting with `K`, e.g. `K1`.
    pub code: String,
    #[serde(default)]
    pub description: String,
}

/// A console action and the keys that trigger it, e.g. `Mod+Shift+Z`
/// (`Mod` is Ctrl, or Cmd on a Mac).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KeyBinding {
    pub action: String,
    pub keys: String,
}

impl Default for Tables {
    fn default() -> Self {
        let w = |skill: char, lose: &str, win: &str| WinningRow {
            skill,
            lose: lose.into(),
            win: win.into(),
        };
        // Winning symbols as DataVolley ships them (every corpus file).
        let winning = vec![
            w('S', "=", "#"),
            w('R', "=", ""),
            w('E', "=", ""),
            w('A', "=/", "#"),
            w('B', "=/", "#"),
            w('D', "=", ""),
            w('F', "=", ""),
        ];
        // A starting set of the common combination codes (industry
        // conventions); descriptions are ours.
        let c = |code: &str, zone: u8, tempo: char, target: &str, description: &str| Combination {
            code: code.into(),
            start_zone: zone,
            tempo,
            target: target.into(),
            description: description.into(),
        };
        let combinations = vec![
            c("X1", 3, 'Q', "C", "Quick in front of the setter"),
            c("X2", 2, 'Q', "C", "Quick behind the setter"),
            c("X7", 3, 'Q', "C", "Quick in the gap"),
            c("XD", 2, 'Q', "C", "Slide"),
            c("X5", 4, 'T', "F", "Fast set to zone 4"),
            c("X6", 2, 'T', "B", "Fast set to zone 2"),
            c("X8", 1, 'T', "B", "Fast set to zone 1"),
            c("XP", 8, 'T', "P", "Fast pipe"),
            c("V5", 4, 'H', "F", "High set to zone 4"),
            c("V6", 2, 'H', "B", "High set to zone 2"),
            c("V8", 1, 'H', "B", "High set to zone 1"),
            c("VP", 8, 'H', "P", "High pipe"),
            c("V3", 3, 'H', "C", "High set to zone 3"),
            c("PP", 3, 'O', "S", "Setter dump"),
            c("P2", 3, 'O', "", "Attack on the second touch"),
            c("PR", 4, 'O', "", "Attack on the first touch"),
        ];
        let k = |code: &str, description: &str| SetterCall {
            code: code.into(),
            description: description.into(),
        };
        let setter_calls = vec![
            k("K1", "Quick in front"),
            k("K2", "Quick behind"),
            k("K7", "Quick in the gap"),
            k("KC", "Slide"),
        ];
        Tables {
            winning,
            combinations,
            setter_calls,
        }
    }
}

impl Tables {
    /// The `[3WINNINGSYMBOLS]` line: per skill in `SRABDEF` order, four
    /// losing then four winning symbols, `~` for none.
    pub fn winning_line(&self) -> String {
        let mut out = String::new();
        for skill in "SRABDEF".chars() {
            let row = self.winning.iter().find(|r| r.skill == skill);
            for set in [row.map(|r| &r.lose), row.map(|r| &r.win)] {
                let chars: Vec<char> = set.map_or(Vec::new(), |s| s.chars().take(4).collect());
                for i in 0..4 {
                    out.push(chars.get(i).copied().unwrap_or('~'));
                }
            }
        }
        out
    }

    /// `[3ATTACKCOMBINATION]` rows: code; start zone; side; tempo;
    /// description; ; colour; start coordinate; target; ;
    pub fn combination_lines(&self) -> Vec<String> {
        self.combinations
            .iter()
            .map(|c| {
                let side = match c.start_zone {
                    4 | 5 | 7 => "L",
                    2 | 1 | 9 => "R",
                    _ => "C",
                };
                format!(
                    "{};{};{side};{};{};;;;{};;",
                    c.code,
                    c.start_zone,
                    c.tempo,
                    c.description.replace(';', ","),
                    c.target
                )
            })
            .collect()
    }

    /// `[3SETTERCALL]` rows: code; ; description; ; colour; …
    pub fn setter_call_lines(&self) -> Vec<String> {
        self.setter_calls
            .iter()
            .map(|k| format!("{};;{};;;;;;;", k.code, k.description.replace(';', ",")))
            .collect()
    }
}

impl Season {
    /// The season's settings; the defaults when none are saved.
    pub fn settings(&self) -> Result<Settings> {
        let json: Option<String> = self
            .conn
            .query_row("SELECT value FROM setting WHERE key = 'season'", [], |r| {
                r.get(0)
            })
            .optional()?;
        Ok(json
            .and_then(|j| serde_json::from_str(&j).ok())
            .unwrap_or_default())
    }

    pub fn save_settings(&mut self, settings: &Settings) -> Result<()> {
        let json =
            serde_json::to_string(settings).map_err(|e| crate::Error::Invalid(e.to_string()))?;
        self.conn.execute(
            "INSERT INTO setting (key, value) VALUES ('season', ?1)
             ON CONFLICT (key) DO UPDATE SET value = excluded.value",
            [json],
        )?;
        Ok(())
    }

    /// The tables a match was created with (the season's at the time).
    pub(crate) fn match_tables(&self, id: MatchId) -> Result<Tables> {
        let json: Option<String> = self.conn.query_row(
            "SELECT tables FROM match WHERE id = ?1",
            [id.to_string()],
            |r| r.get(0),
        )?;
        Ok(json
            .and_then(|j| serde_json::from_str(&j).ok())
            .unwrap_or_default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_winning_line_is_datavolleys() {
        assert_eq!(
            Tables::default().winning_line(),
            "=~~~#~~~=~~~~~~~=/~~#~~~=/~~#~~~=~~~~~~~=~~~~~~~=~~~~~~~"
        );
    }
}
