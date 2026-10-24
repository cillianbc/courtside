//! Clip playlists (PRD F-23): the touches a filter picks, each as a stretch
//! of the match video around its video time (pre- and post-roll from the
//! season settings). Serialised as camelCase JSON; `apps/desktop/src/lib/
//! snapshot.ts` mirrors these types.

use courtside_store::Team as StoreTeam;
use serde::{Deserialize, Serialize};
use vb_engine::Match;
use vb_engine::fold::{Phase, State, fold_events};
use vb_grammar::{Code, Player, Skill, Team, Touch};

use crate::readback;
use crate::snapshot::{Pair, Side};

/// Which touches go in the playlist; every field left empty matches all.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ClipFilter {
    pub team: Option<Side>,
    /// Shirt number (with `team`, that team's player; without, either's).
    pub player: Option<u8>,
    /// Skill letter: S R E A B D F.
    pub skill: Option<char>,
    /// Evaluation symbols, any of which matches (`#+`); empty for all.
    pub evaluations: String,
    /// The touching team's rotation: its setter's zone, 1–6.
    pub rotation: Option<u8>,
    pub phase: Option<ClipPhase>,
    pub set: Option<u8>,
    /// Text the code contains, as written (`D=`, `a12`); empty for all.
    pub text: String,
}

impl ClipFilter {
    /// Whether only `text` is set: a search over every code, not just
    /// touches.
    fn text_only(&self) -> bool {
        *self
            == ClipFilter {
                text: self.text.clone(),
                ..ClipFilter::default()
            }
    }

    /// Whether the touch `t`, made in state `st`, is one the filter picks.
    fn picks(&self, t: &Touch, st: &State) -> bool {
        let evaluations: Vec<char> = self.evaluations.chars().collect();
        self.team.is_none_or(|s| team_of(s) == t.team)
            && self.player.is_none_or(|n| shirt(&t.player) == Some(n))
            && self.skill.is_none_or(|k| t.skill.as_char() == k)
            && (evaluations.is_empty() || t.evaluation().is_some_and(|v| evaluations.contains(&v)))
            && self
                .rotation
                .is_none_or(|r| *st.setter_position.get(t.team) == Some(r))
            && self
                .phase
                .is_none_or(|p| st.phase.map(ClipPhase::from) == Some(p))
            && self.set.is_none_or(|s| st.set == s)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ClipPhase {
    Serve,
    /// Side-out: the receiving team's first contacts.
    Reception,
    /// Break point: everything after the first attack.
    Transition,
}

impl From<Phase> for ClipPhase {
    fn from(p: Phase) -> Self {
        match p {
            Phase::Serve => ClipPhase::Serve,
            Phase::Reception => ClipPhase::Reception,
            Phase::Transition => ClipPhase::Transition,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Clip {
    /// The code's event id, as in the code stream's rows.
    pub id: String,
    pub set: u8,
    /// The set's score when the touch was made.
    pub score: Pair<u8>,
    pub team: Side,
    pub text: String,
    /// The read-back, as in the command bar.
    pub meaning: String,
    /// The code's own video time.
    pub video_ms: u64,
    /// Where the clip starts and ends in the video.
    pub start_ms: u64,
    pub end_ms: u64,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Clips {
    /// In match order.
    pub clips: Vec<Clip>,
    /// Touches the filter picks that have no video time (coded live and
    /// never synced), so the scout knows the list isn't the whole picture.
    pub without_video: usize,
}

/// Seconds of video before and after each code.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Roll {
    pub pre_ms: u64,
    pub post_ms: u64,
}

fn shirt(p: &Player) -> Option<u8> {
    match p {
        Player::Number(d) => d.value().and_then(|v| u8::try_from(v).ok()),
        Player::Unknown => None,
    }
}

fn team_of(s: Side) -> Team {
    match s {
        Side::Home => Team::Home,
        Side::Visiting => Team::Visiting,
    }
}

pub fn build(game: &Match, teams: &(StoreTeam, StoreTeam), f: &ClipFilter, roll: Roll) -> Clips {
    let names = (teams.0.name.as_str(), teams.1.name.as_str());
    let events = game.log.visible();
    let folded = fold_events(game, &events);
    let mut out = Clips::default();
    for (e, st) in events.iter().zip(&folded.states) {
        let Some(code @ Code::Touch(t)) = e.code() else {
            continue;
        };
        if t.skill == Skill::Green {
            continue;
        }
        let text = code.to_string();
        if !f.picks(t, st) || !text.contains(&f.text) {
            continue;
        }
        let Some(at) = e.video_ms else {
            out.without_video += 1;
            continue;
        };
        out.clips.push(Clip {
            id: e.id.0.to_string(),
            set: st.set,
            score: Pair {
                home: st.score.home,
                visiting: st.score.visiting,
            },
            team: Side::from(t.team),
            meaning: readback::read(&text, game, names).meaning,
            text,
            video_ms: at,
            start_ms: at.saturating_sub(roll.pre_ms),
            end_ms: at + roll.post_ms,
        });
    }
    out
}

/// The codes a search in the code log finds (#25), by event id in match
/// order, with or without a video time. Text alone looks through every
/// code (points, substitutions, timeouts); any other field, touches only.
/// Green codes are never found by the touch fields.
pub fn find(game: &Match, f: &ClipFilter) -> Vec<String> {
    let events = game.log.visible();
    let folded = fold_events(game, &events);
    let any_code = f.text_only();
    if any_code && f.text.is_empty() {
        return Vec::new();
    }
    events
        .iter()
        .zip(&folded.states)
        .filter(|(e, st)| match e.code() {
            Some(code @ Code::Touch(t)) if t.skill != Skill::Green || any_code => {
                (any_code || f.picks(t, st)) && code.to_string().contains(&f.text)
            }
            Some(code) => any_code && code.to_string().contains(&f.text),
            None => false,
        })
        .map(|(e, _)| e.id.0.to_string())
        .collect()
}
