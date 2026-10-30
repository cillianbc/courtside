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
/// scout leaves the set out unless someone else set (`3E 8#`). With
/// `server` (the team due to serve and its player in zone 1), a rally's
/// first token gets the serve filled in (`predicted_serve`).
pub fn preview(
    text: &str,
    game: &Match,
    names: (&str, &str),
    before: Option<&Touch>,
    defaults: &Defaults,
    setters: Option<Sides<Option<u8>>>,
    server: Option<(Team, u8)>,
) -> Preview {
    let mut codes: Vec<Readback> = Vec::new();
    let mut last = before.cloned();
    for (n, token) in text.split_whitespace().enumerate() {
        let named = setters.and_then(|s| setters_call(token, s));
        // A rally's first touch, before any of this line's.
        let starts_rally = last.is_none()
            && !codes
                .iter()
                .any(|c| matches!(Code::parse(&c.text), Code::Touch(_) | Code::Point { .. }));
        let served = server
            .filter(|_| starts_rally && named.is_none())
            .and_then(|s| predicted_serve(token, s));
        let first_code = codes.len();
        let typed = served.as_deref().or(named.as_deref()).unwrap_or(token);
        match expand(typed, last.as_ref(), defaults) {
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
                    // The set before an attack takes its target from the
                    // attack's combination, unless one was typed.
                    if let Code::Touch(t) = code
                        && t.skill == Skill::Attack
                        && let Some(target) = t
                            .combination()
                            .and_then(|c| defaults.combination(&c)?.target)
                        && let Some(prev) = codes.last_mut()
                        && let Code::Touch(set) = Code::parse(&prev.text)
                        && set.skill == Skill::Set
                        && set.team == t.team
                        && set.target().is_none()
                    {
                        let set = set.with_target(target);
                        let mut r = read(&set.to_string(), game, names);
                        r.typed = std::mem::take(&mut prev.typed);
                        r.token = prev.token;
                        r.filled = std::mem::take(&mut prev.filled);
                        r.filled.push("target from the combination".into());
                        *prev = r;
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
                if served.is_some()
                    && let Some(r) = codes.get_mut(first_code)
                {
                    r.filled.insert(0, "serve by the server in zone 1".into());
                }
            }
        }
    }
    let ok = !codes.is_empty() && codes.iter().all(|c| c.error.is_none());
    Preview { codes, ok }
}

/// The serve a rally's first token stands for, as a token `expand` reads,
/// when the scout leaves it to the rotation (GitHub #12): an evaluation
/// alone, with or without the serve's type (`+`, `Q-`, `S`), is the server's
/// serve; the receiving team's player with no skill or `R` (`7-`, `a3R#`)
/// is the reception of it, the serve's evaluation derived from it. Anything
/// else is read as typed, so a wrong server stays the validator's to flag.
fn predicted_serve(token: &str, (team, server): (Team, u8)) -> Option<String> {
    let t = team.as_char();
    let (prefix, rest) = match token.chars().next()? {
        'a' => (Some(Team::Visiting), &token[1..]),
        '*' => (Some(Team::Home), &token[1..]),
        _ => (None, token),
    };
    if rest.contains('.') {
        return None;
    }
    let digits = rest.chars().take_while(char::is_ascii_digit).count();
    let tail = rest[digits..].to_ascii_uppercase();
    if digits == 0 {
        // `+`, `S+`, `Q-`: the serve itself, by the server.
        let body = tail.strip_prefix('S').unwrap_or(&tail);
        let mut chars = body.chars();
        let ty = chars.clone().next().filter(|c| "HMQTUNO".contains(*c));
        if ty.is_some() {
            chars.next();
        }
        let ev = chars.as_str();
        // An evaluation, or `S` itself: `aT` is a timeout, not a serve type.
        let ok = prefix.is_none_or(|p| p == team)
            && if ev.is_empty() {
                tail.starts_with('S')
            } else {
                ev.len() == 1 && "=/-!+#".contains(ev)
            };
        return ok.then(|| format!("{t}{server:02}S{body}"));
    }
    let receiving = prefix.unwrap_or(Team::Home) != team;
    let skill = tail.chars().next().filter(char::is_ascii_alphabetic);
    let reception =
        matches!(skill, None | Some('R')) || tail.starts_with(|c: char| "HMQTUNO".contains(c));
    (receiving && reception).then(|| format!("{t}{server:02}S.{token}"))
}

/// `K7#`, `aK1`: a setter call typed without a player is the team's
/// setter's set (VolleyStation's shorthand); the token with the setter's
/// number put in, or `None` when it isn't one or no setter is known.
fn setters_call(token: &str, setters: Sides<Option<u8>>) -> Option<String> {
    let (team, rest) = match token.strip_prefix('a') {
        Some(r) => (Team::Visiting, r),
        None => (Team::Home, token.strip_prefix('*').unwrap_or(token)),
    };
    if !rest.starts_with(['K', 'k']) || rest.len() < 2 || rest.contains('.') {
        return None;
    }
    let n = (*setters.get(team))?;
    Some(format!("{}{n:02}{rest}", team.as_char()))
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
    // DataVolley's P combinations are attacks on the first or second touch
    // (PP setter dump, P2 second touch, PR first touch): no set before them.
    if attack.combination().is_some_and(|c| c.starts_with('P')) {
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
        Code::Comment(c) => (format!("Comment: {}", c.trim_start_matches('>')), None),
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

/// The evaluation in words for this skill: where DataVolley's standard
/// table means something particular (an attack's `/` is blocked, a block's
/// a net touch), that; otherwise the general word.
fn evaluation_word(skill: Skill, c: char) -> Option<&'static str> {
    let particular = match (skill, c) {
        (Skill::Serve, '#') => "ace",
        (Skill::Attack, '#') => "kill",
        (Skill::Attack, '!') => "blocked back for a re-attack",
        (Skill::Attack, '/') => "blocked",
        (Skill::Block, '#') => "stuff block",
        (Skill::Block, '/') => "net touch",
        (Skill::Dig, '/') => "straight back over",
        _ => return evaluation_name(c),
    };
    Some(particular)
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

/// The extended part in words: the subtype, number of players and
/// special, with the meanings openvolley's `datavolley` reader gives them
/// (`R/plays.R`). An attack's
/// and a serve's special depend on how it ended.
fn extras(t: &Touch, e: char) -> Vec<String> {
    let subtype = t.skill_subtype().and_then(|c| {
        Some(match (t.skill, c) {
            (Skill::Attack, 'H') => "hard hit",
            (Skill::Attack, 'P') => "soft spike",
            (Skill::Attack, 'T') => "tip",
            (Skill::Block, 'A') => "assist",
            (Skill::Block, 'T') => "attempt",
            (Skill::Block, 'P') => "on a soft spike",
            (Skill::Reception, 'L') => "on the left",
            (Skill::Reception, 'R') => "on the right",
            (Skill::Reception, 'W') => "low",
            (Skill::Reception, 'O') => "overhand",
            (Skill::Reception, 'M') => "middle line",
            (Skill::Set, '1') => "one hand",
            (Skill::Set, '2') => "two hands",
            (Skill::Set, '3') => "bump",
            (Skill::Set, '4') => "other hands",
            (Skill::Set, '5') => "underhand",
            (Skill::Dig, 'S') => "on a spike",
            (Skill::Dig, 'C') => "spike cover",
            (Skill::Dig, 'B') => "after the block",
            (Skill::Dig, 'E') => "emergency",
            (Skill::Dig, 'T') => "on a tip",
            (Skill::Dig, 'P') => "on a soft spike",
            _ => return None,
        })
    });
    let players = t.num_players().and_then(|c| {
        Some(match (t.skill, c) {
            (Skill::Attack | Skill::Block, '0') => "no block",
            (Skill::Attack | Skill::Block, '1') => "one blocker",
            (Skill::Attack | Skill::Block, '2') => "two blockers",
            (Skill::Attack | Skill::Block, '3') => "three blockers",
            (Skill::Attack | Skill::Block, '4') => "hole in the block",
            (Skill::Reception, '1') => "two receiving, the left one",
            (Skill::Reception, '2') => "two receiving, the right one",
            (Skill::Reception, '3') => "three receiving, the left one",
            (Skill::Reception, '4') => "three receiving, the centre one",
            (Skill::Reception, '5') => "three receiving, the right one",
            (Skill::Reception, '6') => "four receiving, the left one",
            (Skill::Reception, '7') => "four receiving, centre-left",
            (Skill::Reception, '8') => "four receiving, centre-right",
            (Skill::Reception, '9') => "four receiving, the right one",
            _ => return None,
        })
    });
    let special = t.special().and_then(|c| {
        Some(match (t.skill, e, c) {
            (Skill::Attack, '=', 'S') => "out at the side",
            (Skill::Attack, '=', 'O') => "out long",
            (Skill::Attack, '=', 'N') => "in the net",
            (Skill::Attack, '=', 'I') => "net touch",
            (Skill::Attack, '=', 'A') => "antenna",
            (Skill::Attack, '#', 'S') => "off the block, out at the side",
            (Skill::Attack, '#', 'O') => "off the block, out long",
            (Skill::Attack, '#', 'F') => "off the block to the floor",
            (Skill::Attack, '#', 'X') => "straight to the floor",
            (Skill::Attack, _, 'C') => "block control",
            (Skill::Attack | Skill::Serve, _, 'N') => "off the net",
            (Skill::Block, _, 'S') => "out at the side",
            (Skill::Block, _, 'O') => "out long",
            (Skill::Block, _, 'F') => "to the floor",
            (Skill::Block, _, 'X') => "between the hands",
            (Skill::Block, _, 'N') => "hands and net",
            (Skill::Block | Skill::Set, _, 'I') => "net touch",
            (Skill::Block, _, 'A') => "antenna",
            (Skill::Block, _, 'P') => "no jump",
            (Skill::Block, _, 'T') => "position error",
            (Skill::Serve, '=', 'O') => "out long",
            (Skill::Serve, '=', 'L') => "out left",
            (Skill::Serve, '=', 'R') => "out right",
            (Skill::Reception | Skill::Dig | Skill::Freeball, _, 'U') => "unplayable",
            (Skill::Reception | Skill::Dig | Skill::Freeball, _, 'X') => "body error",
            (Skill::Reception | Skill::Dig | Skill::Freeball, _, 'P') => "position error",
            (Skill::Reception | Skill::Dig | Skill::Freeball, _, 'E') => "lack of effort",
            (Skill::Dig, _, 'F') => "to the floor",
            (Skill::Dig, _, 'O') => "out",
            (Skill::Set, _, 'U') => "unhittable",
            (_, _, 'Z') => "referee call",
            _ => return None,
        })
    });
    let unknown = |what: &str, c: Option<char>, known: bool| {
        c.filter(|_| !known).map(|c| format!("{what} {c}?"))
    };
    let mut out: Vec<String> = [subtype, players, special]
        .into_iter()
        .flatten()
        .map(str::to_owned)
        .collect();
    // A letter with no meaning for this skill is said as typed, so it shows.
    out.extend(unknown("extra", t.skill_subtype(), subtype.is_some()));
    out.extend(unknown("extra", t.num_players(), players.is_some()));
    out.extend(unknown("extra", t.special(), special.is_some()));
    out
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
    let Some(name) = evaluation_word(t.skill, e) else {
        return (parts.join(", "), Some(format!("Unknown evaluation {e}.")));
    };
    parts.push(name.into());
    if let Some(c) = t.combination() {
        parts.push(format!("call {c}"));
    }
    parts.extend(extras(t, e));
    match game.winning_symbols.outcome(t.skill, e) {
        Some(Outcome::Win) => parts.push("wins the point".into()),
        Some(Outcome::Lose) => parts.push("loses the point".into()),
        None => {}
    }
    (parts.join(", "), None)
}
