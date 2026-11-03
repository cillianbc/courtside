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
    /// Each touch found gets this rating (`#+!-/=`).
    Rating { evaluation: char },
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
    /// The scout left it out (unticked in the preview).
    pub left: bool,
    /// Other codes that changed with it, as a hand edit changes them.
    pub followed: Vec<Followed>,
}

/// A code the search didn't find that a change wrote, corrected or took
/// out: the other half of a pair, a set's target, a rally's ending, a
/// rotation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Followed {
    /// Its event id now; `None` when it was taken out.
    pub id: Option<String>,
    pub before: Option<String>,
    pub after: Option<String>,
    pub kind: FollowKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum FollowKind {
    /// A touch: the other half of a pair, or a set's target.
    Paired,
    Point,
    Green,
    Rotation,
    Other,
}

impl FollowKind {
    pub fn of(code: &vb_grammar::Code) -> Self {
        use vb_grammar::{Code, Skill};
        match code {
            Code::Touch(t) if t.skill == Skill::Green => FollowKind::Green,
            Code::Touch(_) => FollowKind::Paired,
            Code::Point { .. } => FollowKind::Point,
            Code::SetterPosition { .. } => FollowKind::Rotation,
            _ => FollowKind::Other,
        }
    }
}
