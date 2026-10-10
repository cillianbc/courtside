//! File-level checks for `courtside validate`. Match rules (score,
//! rotation, substitutions) need vb-engine and arrive in phase 2.

use std::fmt;
use std::path::Path;

use dvw_codec::text::decode_embedded;
use dvw_codec::{Dvw, RawVsm};
use vb_grammar::Code;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    Warning,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Issue {
    pub severity: Severity,
    /// 1-based file line, when the issue has one.
    pub line: Option<usize>,
    pub message: String,
}

impl fmt::Display for Issue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let severity = match self.severity {
            Severity::Warning => "warning",
            Severity::Error => "error",
        };
        match self.line {
            Some(line) => write!(f, "{line}: {severity}: {}", self.message),
            None => write!(f, "{severity}: {}", self.message),
        }
    }
}

fn issue(severity: Severity, line: Option<usize>, message: impl Into<String>) -> Issue {
    Issue {
        severity,
        line,
        message: message.into(),
    }
}

pub fn validate(path: &Path, bytes: &[u8]) -> Vec<Issue> {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(str::to_ascii_lowercase);
    match ext.as_deref() {
        Some("dvw") => validate_dvw(bytes),
        Some("vsm") => validate_vsm(bytes),
        _ => vec![issue(
            Severity::Error,
            None,
            "unsupported file type; expected .dvw or .vsm",
        )],
    }
}

fn validate_vsm(bytes: &[u8]) -> Vec<Issue> {
    match RawVsm::parse(bytes) {
        Err(e) => vec![issue(Severity::Error, None, e.to_string())],
        Ok(vsm) if vsm.to_bytes() != bytes => vec![issue(
            Severity::Warning,
            None,
            "not in VolleyStation's compact JSON layout; re-saving will change formatting",
        )],
        Ok(_) => Vec::new(),
    }
}

fn validate_dvw(bytes: &[u8]) -> Vec<Issue> {
    let dvw = match Dvw::parse(bytes) {
        Ok(d) => d,
        Err(e) => return vec![issue(Severity::Error, None, e.to_string())],
    };
    let mut issues = Vec::new();
    let raw = dvw.raw();

    for name in [
        "3DATAVOLLEYSCOUT",
        "3MATCH",
        "3TEAMS",
        "3PLAYERS-H",
        "3PLAYERS-V",
        "3SCOUT",
    ] {
        if raw.section(name).is_none() {
            issues.push(issue(
                Severity::Error,
                None,
                format!("missing [{name}] section"),
            ));
        }
    }
    let mut seen = std::collections::HashSet::new();
    for section in &raw.sections {
        if !seen.insert(section.name.as_str()) {
            issues.push(issue(
                Severity::Warning,
                raw.header_line(&section.name),
                format!(
                    "[{}] appears more than once; only the first is read",
                    section.name
                ),
            ));
        }
    }

    match dvw.to_bytes() {
        Ok(out) if out == bytes => {}
        Ok(_) => issues.push(issue(
            Severity::Error,
            None,
            "file does not round-trip through Courtside; please report it with the file",
        )),
        Err(e) => issues.push(issue(Severity::Error, None, e.to_string())),
    }

    if dvw.teams.len() != 2 {
        issues.push(issue(
            Severity::Error,
            raw.header_line("3TEAMS"),
            format!("expected 2 teams, found {}", dvw.teams.len()),
        ));
    }
    if dvw.match_date().is_none() {
        issues.push(issue(
            Severity::Warning,
            raw.header_line("3MATCH").map(|l| l + 1),
            format!(
                "cannot determine the match date from {:?}",
                dvw.match_info.date_text()
            ),
        ));
    }

    for (section, roster) in [
        ("3PLAYERS-H", &dvw.players_home),
        ("3PLAYERS-V", &dvw.players_visiting),
    ] {
        let first = raw.header_line(section).map_or(0, |l| l + 1);
        let mut numbers = std::collections::HashSet::new();
        for (i, p) in roster.iter().enumerate() {
            let line = Some(first + i);
            match p.number() {
                None => issues.push(issue(
                    Severity::Error,
                    line,
                    "player without a shirt number",
                )),
                Some(n) if !numbers.insert(n) => issues.push(issue(
                    Severity::Error,
                    line,
                    format!("shirt number {n} listed twice"),
                )),
                Some(_) => {}
            }
            for (plain, embedded, what) in [
                (9, 17, "last name"),
                (10, 18, "first name"),
                (11, 19, "nickname"),
            ] {
                if let Some(text) = decode_embedded(p.0.get(embedded))
                    && !text.is_empty()
                    && text != p.0.get(plain)
                {
                    issues.push(issue(
                        Severity::Warning,
                        line,
                        format!(
                            "{what} differs from its Unicode copy (file edited by another program?)"
                        ),
                    ));
                }
            }
        }
    }

    let first = raw.header_line("3SCOUT").map_or(0, |l| l + 1);
    for (i, line) in dvw.scout.iter().enumerate() {
        if line.code().is_empty() {
            continue;
        }
        if let Code::Unrecognised(code) = Code::parse(line.code()) {
            issues.push(issue(
                Severity::Warning,
                Some(first + i),
                format!("unrecognised code {code:?}"),
            ));
        }
    }
    issues.sort_by_key(|i| (i.line.unwrap_or(0), std::cmp::Reverse(i.severity)));
    issues
}

#[cfg(test)]
mod tests {
    use super::*;

    const GOOD: &[u8] = b"[3DATAVOLLEYSCOUT]\nGENERATOR-PRG: Data Volley\n[3MATCH]\n03/04/2026;;;;;;;;1252;1;Z;\n[3TEAMS]\nT1;Home;0;;;;\nT2;Away;0;;;;\n[3PLAYERS-H]\n0;1;1;;;;;;H1;One;A;;;2;;\n[3PLAYERS-V]\n1;2;1;;;;;;V2;Two;B;;;2;;\n[3SCOUT]\n*01SH#;;;;;;;;;;;;;;;;;;;;;;;;;;\n";

    #[test]
    fn clean_file_has_no_issues() {
        assert_eq!(validate(Path::new("m.dvw"), GOOD), []);
    }

    #[test]
    fn reports_problems_with_lines() {
        let bad = b"[3DATAVOLLEYSCOUT]\n\
GENERATOR-PRG: Data Volley\n\
[3MATCH]\n\
03/04/2026;;;;;;;;1252;1;Z;\n\
[3TEAMS]\n\
T1;Home;0;;;;\n\
T2;Away;0;;;;\n\
[3PLAYERS-H]\n\
0;1;1;;;;;;H1;One;A;;;2;;\n\
0;1;2;;;;;;H2;Dup;C;;;2;;;;\x0f24F6E65;;\n\
[3PLAYERS-V]\n\
1;2;1;;;;;;V2;Two;B;;;2;;\n\
[3SCOUT]\n\
*01SH#;;;;;;;;;;;;;;;;;;;;;;;;;;\n\
>ROT\n";
        let text: Vec<String> = validate(Path::new("m.DVW"), bad)
            .iter()
            .map(ToString::to_string)
            .collect();
        assert_eq!(
            text,
            [
                "10: error: shirt number 1 listed twice",
                "10: warning: last name differs from its Unicode copy (file edited by another program?)",
                "15: warning: unrecognised code \">ROT\"",
            ]
        );
    }

    #[test]
    fn missing_sections_and_bad_types() {
        let issues = validate(Path::new("m.dvw"), b"[3MATCH]\n;;;;;;;;1252;\n");
        assert!(
            issues
                .iter()
                .any(|i| i.message == "missing [3SCOUT] section")
        );
        assert_eq!(
            validate(Path::new("m.txt"), b"")[0].severity,
            Severity::Error
        );
        assert!(validate(Path::new("m.vsm"), b"{\"a\":1}").is_empty());
        assert_eq!(
            validate(Path::new("m.vsm"), b"{\"a\": 1}")[0].severity,
            Severity::Warning
        );
        assert_eq!(
            validate(Path::new("m.vsm"), b"{")[0].severity,
            Severity::Error
        );
    }

    /// Every corpus file validates without errors or warnings.
    #[test]
    fn corpus_is_clean() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../testdata");
        if !root.is_dir() {
            assert!(
                std::env::var("COURTSIDE_REQUIRE_CORPUS").as_deref() != Ok("1"),
                "corpus required but missing"
            );
            return;
        }
        let mut stack = vec![root];
        let mut checked = 0;
        let mut problems = Vec::new();
        while let Some(dir) = stack.pop() {
            for entry in std::fs::read_dir(&dir).unwrap() {
                let path = entry.unwrap().path();
                if path.is_dir() {
                    stack.push(path);
                } else if matches!(
                    path.extension().and_then(|e| e.to_str()),
                    Some("dvw" | "vsm")
                ) {
                    checked += 1;
                    for i in validate(&path, &std::fs::read(&path).unwrap()) {
                        problems.push(format!("{}:{i}", path.display()));
                    }
                }
            }
        }
        assert!(checked > 0);
        assert!(problems.is_empty(), "{}", problems.join("\n"));
    }
}
