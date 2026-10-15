//! Team and roster management as the UI sees it: camelCase views and the
//! forms it sends back. `apps/desktop/src/lib/teams.ts` mirrors these.

use courtside_store::{Player, PlayerInput, SeasonPlayer, Team, TeamImport, TeamInput};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TeamDetail {
    pub id: String,
    pub name: String,
    pub code: String,
    pub coach: String,
    pub assistant: String,
    pub matches: usize,
    pub players: Vec<PlayerDetail>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlayerDetail {
    /// Empty for a player not saved yet.
    #[serde(default)]
    pub id: String,
    pub number: u8,
    #[serde(default)]
    pub code: String,
    pub last_name: String,
    #[serde(default)]
    pub first_name: String,
    #[serde(default)]
    pub nickname: String,
    /// `.dvw` role: 1 libero, 2 outside, 3 opposite, 4 middle, 5 setter.
    #[serde(default)]
    pub role: Option<u8>,
    #[serde(default)]
    pub libero: bool,
    #[serde(default)]
    pub captain: bool,
    #[serde(default)]
    pub foreign: bool,
    #[serde(default)]
    pub birth_date: Option<String>,
    #[serde(default)]
    pub height_cm: Option<u16>,
}

/// A team's details as the editor sends them.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct TeamForm {
    pub name: String,
    pub code: String,
    pub coach: String,
    pub assistant: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlayerListItem {
    pub player: PlayerDetail,
    pub team_id: String,
    pub team_name: String,
    pub matches: usize,
}

impl From<Player> for PlayerDetail {
    fn from(p: Player) -> Self {
        PlayerDetail {
            id: p.id,
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
        }
    }
}

impl From<Team> for TeamDetail {
    fn from(t: Team) -> Self {
        TeamDetail {
            id: t.id,
            name: t.name,
            code: t.code,
            coach: t.coach,
            assistant: t.assistant,
            matches: t.matches,
            players: t.players.into_iter().map(PlayerDetail::from).collect(),
        }
    }
}

impl From<SeasonPlayer> for PlayerListItem {
    fn from(p: SeasonPlayer) -> Self {
        PlayerListItem {
            player: p.player.into(),
            team_id: p.team_id,
            team_name: p.team_name,
            matches: p.matches,
        }
    }
}

impl From<&TeamForm> for TeamInput {
    fn from(f: &TeamForm) -> Self {
        TeamInput {
            name: f.name.clone(),
            code: f.code.clone(),
            coach: f.coach.clone(),
            assistant: f.assistant.clone(),
        }
    }
}

impl From<&PlayerDetail> for PlayerInput {
    fn from(p: &PlayerDetail) -> Self {
        PlayerInput {
            id: (!p.id.is_empty()).then(|| p.id.clone()),
            number: p.number,
            code: p.code.clone(),
            last_name: p.last_name.clone(),
            first_name: p.first_name.clone(),
            nickname: p.nickname.clone(),
            role: p.role,
            // A libero by role is a libero.
            libero: p.libero || p.role == Some(1),
            captain: p.captain,
            foreign: p.foreign,
            birth_date: p.birth_date.clone().filter(|d| !d.is_empty()),
            height_cm: p.height_cm,
        }
    }
}

/// What importing a team file did, for the message the scout sees.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TeamImportView {
    pub team_id: String,
    pub team_name: String,
    pub created: bool,
    pub players_added: usize,
    pub players_kept: usize,
}

impl From<TeamImport> for TeamImportView {
    fn from(t: TeamImport) -> Self {
        TeamImportView {
            team_id: t.team_id,
            team_name: t.team_name,
            created: t.created,
            players_added: t.players_added,
            players_kept: t.players_kept,
        }
    }
}
