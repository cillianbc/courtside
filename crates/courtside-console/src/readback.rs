//! The command bar's read-back: what a typed code means, in words, before
//! the scout presses Enter.

use serde::Serialize;
use vb_engine::Match;
use vb_engine::model::{Outcome, Sides};
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
    /// Which space-separated token of the line it came from.
    #[serde(skip)]
    pub token: usize,
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
/// `before` is the last touch of the open rally. With `setters` (each
/// team's setter's shirt number), an attack straight after its team's
/// reception, dig or free ball gets the setter's set in front of it: the
/// scout leaves the set out unless someone else set (`3E 8#`).
pub fn preview(
    text: &str,
    game: &Match,
    names: (&str, &str),
    before: Option<&Touch>,
    defaults: &Defaults,
    setters: Option<Sides<Option<u8>>>,
) -> Preview {
    let mut codes = Vec::new();
    let mut last = before.cloned();
    for (n, token) in text.split_whitespace().enumerate() {
        match expand(token, last.as_ref(), defaults) {
            Err(e) => codes.push(Readback {
                typed: token.to_owned(),
                text: token.to_owned(),
                filled: Vec::new(),
                meaning: String::new(),
                error: Some(e),
                token: n,
            }),
            Ok(x) => {
                for (i, code) in x.codes.iter().enumerate() {
                    if let Code::Touch(t) = code
                        && let Some(set) =
                            setters.and_then(|s| inferred_set(t, last.as_ref(), s, defaults))
                    {
                        let mut r = read(&set.to_string(), game, names);
                        r.typed = token.to_owned();
                        r.token = n;
                        r.filled = vec!["set by the team's setter".into()];
                        codes.push(r);
                        last = Some(set);
                    }
                    let mut r = read(&code.to_string(), game, names);
                    r.typed = token.to_owned();
                    r.token = n;
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

/// The set the scout left out before `attack`: its team's setter's, when
/// the touch before is the same team's reception, dig or free ball and
/// neither it nor the attack is the setter's own (a second-ball attack, or
/// the setter took the first ball and someone else set).
fn inferred_set(
    attack: &Touch,
    before: Option<&Touch>,
    setters: Sides<Option<u8>>,
    d: &Defaults,
) -> Option<Touch> {
    let b = before?;
    if attack.skill != Skill::Attack
        || b.team != attack.team
        || !matches!(b.skill, Skill::Reception | Skill::Dig | Skill::Freeball)
    {
        return None;
    }
    let setter = (*setters.get(attack.team))?;
    let shirt = |p: &Player| match p {
        Player::Number(d) => d.value().and_then(|v| u8::try_from(v).ok()),
        Player::Unknown => None,
    };
    if shirt(&attack.player) == Some(setter) || shirt(&b.player) == Some(setter) {
        return None;
    }
    // A set takes the type of the attack after it (DataVolley's rule).
    let code = format!(
        "{}{setter:02}E{}{}",
        attack.team.as_char(),
        attack.skill_type().unwrap_or(d.types[2]),
        d.evaluations[2]
    );
    match Code::parse(&code) {
        Code::Touch(t) => Some(t),
        _ => None,
    }
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
        token: 0,
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
