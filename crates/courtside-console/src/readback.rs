//! The command bar's read-back: what a typed code means, in words, before
//! the scout presses Enter.

use serde::Serialize;
use vb_engine::Match;
use vb_engine::model::Outcome;
use vb_grammar::code::Digits;
use vb_grammar::expand::{Defaults, expand};
use vb_grammar::{Code, Player, Skill, Team, Touch};

/// One code of the command line, read back.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Readback {
    /// What the scout typed (a dot code gives two read-backs for one).
    pub typed: String,
    /// The full code it becomes.
    pub text: String,
    /// What expansion filled in ("skill predicted", "type from the
    /// serve" …), on the first code of a token.
    pub filled: Vec<String>,
    /// What the code means; empty when it can't be read.
    pub meaning: String,
    /// Why the code can't be read, or what's missing.
    pub error: Option<String>,
}

/// What the command line holds: codes separated by spaces.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Preview {
    pub codes: Vec<Readback>,
    /// Every code can be read, so Enter will add them.
    pub ok: bool,
}

/// Reads back the command line: each token expanded (shorthand, dot
/// codes) with the rally so far as context, then each code explained.
/// `before` is the last touch of the open rally.
pub fn preview(
    text: &str,
    game: &Match,
    names: (&str, &str),
    before: Option<&Touch>,
    defaults: &Defaults,
) -> Preview {
    let mut codes = Vec::new();
    let mut last = before.cloned();
    for token in text.split_whitespace() {
        match expand(token, last.as_ref(), defaults) {
            Err(e) => codes.push(Readback {
                typed: token.to_owned(),
                text: token.to_owned(),
                filled: Vec::new(),
                meaning: String::new(),
                error: Some(e),
            }),
            Ok(x) => {
                for (i, code) in x.codes.iter().enumerate() {
                    let mut r = read(&code.to_string(), game, names);
                    r.typed = token.to_owned();
                    if i == 0 {
                        r.filled.clone_from(&x.filled);
                    }
                    codes.push(r);
                    if let Code::Touch(t) = code {
                        last = Some(t.clone());
                    } else if matches!(code, Code::Point { .. }) {
                        last = None;
                    }
                }
            }
        }
    }
    let ok = !codes.is_empty() && codes.iter().all(|c| c.error.is_none());
    Preview { codes, ok }
}

/// Reads one code. An error means Enter would reject it.
pub fn read(text: &str, game: &Match, names: (&str, &str)) -> Readback {
    let team_name = |t: Team| match t {
        Team::Home => names.0,
        Team::Visiting => names.1,
    };
    let code = Code::parse(text);
    let (meaning, error) = match &code {
        Code::Unrecognised(_) => (String::new(), Some(why_unreadable(text))),
        Code::Touch(t) => touch(t, game, team_name(t.team)),
        Code::Point { team, .. } => (format!("Point {}", team_name(*team)), None),
        Code::SetterPosition { team, position, .. } => (
            format!("{} setter to zone {}", team_name(*team), number(position)),
            None,
        ),
        Code::SetterPlayer { team, player, .. } => (
            format!("{} setter is #{}", team_name(*team), number(player)),
            None,
        ),
        Code::Substitution {
            team, out, r#in, ..
        } => (
            format!(
                "Substitution {}: #{} off, #{} on",
                team_name(*team),
                number(out),
                number(r#in)
            ),
            None,
        ),
        Code::Timeout { team, .. } => (format!("Timeout {}", team_name(*team)), None),
        Code::SetEnd { set, .. } => (format!("End of set {}", number(set)), None),
    };
    Readback {
        typed: text.to_owned(),
        text: text.to_owned(),
        filled: Vec::new(),
        meaning,
        error,
    }
}

fn number(d: &Digits) -> String {
    d.value()
        .map_or_else(|| d.as_str().to_owned(), |v| v.to_string())
}

fn why_unreadable(text: &str) -> String {
    let mut chars = text.chars();
    match (chars.next(), chars.next()) {
        (Some('*' | 'a'), None) => {
            "Add the shirt number, or T (timeout), c (substitution), z (setter zone) or p (point)."
                .into()
        }
        (Some('*' | 'a'), Some(_)) => "That doesn't fit the code layout.".into(),
        _ => "Start with * (home) or a (visiting).".into(),
    }
}

fn skill_name(s: Skill) -> &'static str {
    match s {
        Skill::Serve => "serve",
        Skill::Reception => "reception",
        Skill::Set => "set",
        Skill::Attack => "attack",
        Skill::Block => "block",
        Skill::Dig => "dig",
        Skill::Freeball => "freeball",
        Skill::Green => "point or error, no touch",
    }
}

fn type_name(c: char) -> Option<&'static str> {
    Some(match c {
        'H' => "high",
        'M' => "medium",
        'Q' => "quick",
        'T' => "tense",
        'U' => "super",
        'N' => "fast",
        'O' => "other",
        _ => return None,
    })
}

fn evaluation_name(c: char) -> Option<&'static str> {
    Some(match c {
        '#' => "perfect",
        '+' => "positive",
        '!' => "ok",
        '-' => "poor",
        '/' => "poor, ball over",
        '=' => "error",
        _ => return None,
    })
}

fn touch(t: &Touch, game: &Match, team: &str) -> (String, Option<String>) {
    let who = match &t.player {
        Player::Number(d) => format!("#{}", number(d)),
        Player::Unknown => "unknown player".into(),
    };
    let mut parts = vec![format!("{team} {who} {}", skill_name(t.skill))];
    if let Some(name) = t.skill_type().and_then(type_name) {
        parts.push(name.into());
    }
    let Some(e) = t.evaluation() else {
        return (
            parts.join(", "),
            Some("Add the evaluation (# + ! - / =).".into()),
        );
    };
    let Some(name) = evaluation_name(e) else {
        return (parts.join(", "), Some(format!("Unknown evaluation {e}.")));
    };
    parts.push(name.into());
    if let Some(c) = t.combination() {
        parts.push(format!("call {c}"));
    }
    match game.winning_symbols.outcome(t.skill, e) {
        Some(Outcome::Win) => parts.push("wins the point".into()),
        Some(Outcome::Lose) => parts.push("loses the point".into()),
        None => {}
    }
    (parts.join(", "), None)
}
