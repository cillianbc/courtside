//! Parser for the DataVolley positional scouting code, with dialect tables.
//! Default expansion and compound (dot) codes come with the coding console.

pub mod code;
pub mod dialect;

pub use code::{Code, Player, Skill, Team, Touch};
pub use dialect::{Dialect, EvaluationTable};
