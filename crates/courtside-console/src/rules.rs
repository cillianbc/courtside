//! Find and replace in the code log (#25): one change made to every code a
//! search finds. Serialised as camelCase JSON; `apps/desktop/src/lib/
//! snapshot.ts` mirrors these types.

use serde::{Deserialize, Serialize};

/// What happens to each code the search finds.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase", tag = "kind")]
pub enum Rule {
    /// The search's code text, wherever it is in the code, becomes `with`.
    Replace { with: String },
    /// `code` goes in before or after the code, typed as on the command
    /// line but without the team: that is the found touch's team or the
    /// other one (`$$D` for a dig by nobody in particular).
    Add {
        code: String,
        place: Place,
        team: RuleTeam,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Place {
    Before,
    After,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RuleTeam {
    Same,
    Other,
}

/// The rule on one code found.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RuleChange {
    /// The found code's event id.
    pub id: String,
    /// The found code as it was.
    pub code: String,
    /// What the rule wrote: the code it became, or the codes added.
    pub result: Vec<String>,
    /// Why the rule left this code alone.
    pub error: Option<String>,
}
