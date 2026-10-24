//! Problems reported back to us: a log file, "Report a problem" (a folder
//! the scout emails), and crash reports to Sentry when the build has a DSN
//! and the scout said yes on this computer.
//!
//! Match files hold real players' names, so nothing leaves the computer by
//! itself except a crash report the scout agreed to, and that carries no
//! codes, names or file paths: the event is scrubbed before it goes.

use std::borrow::Cow;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};

/// Where "Report a problem" addresses its email.
pub const REPORT_TO: &str = "cillianbc@gmail.com";

/// The Sentry project, set when the app is built (`SENTRY_DSN=… pnpm tauri
/// build`). Without it the app never sends a crash report.
const DSN: Option<&str> = option_env!("SENTRY_DSN");

/// The log file's name in the app's log folder (`courtside.log`).
pub const LOG_NAME: &str = "courtside";

/// The scout's answer on this computer, read by every crash report.
static CONSENT: AtomicBool = AtomicBool::new(false);

/// This computer's choice about crash reports (`privacy.json` in the app's
/// config folder).
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Privacy {
    pub crash_reports: bool,
    /// The scout has been asked once.
    pub asked: bool,
}

/// What the UI shows: the choice, and whether this build can send at all.
#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PrivacyView {
    pub crash_reports: bool,
    pub asked: bool,
    pub available: bool,
}

fn privacy_file(app: &AppHandle) -> Option<PathBuf> {
    app.path()
        .app_config_dir()
        .ok()
        .map(|d| d.join("privacy.json"))
}

pub fn load_privacy(app: &AppHandle) -> Privacy {
    privacy_file(app)
        .and_then(|f| std::fs::read(f).ok())
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or_default()
}

pub fn view(p: Privacy) -> PrivacyView {
    PrivacyView {
        crash_reports: p.crash_reports,
        asked: p.asked,
        available: DSN.is_some(),
    }
}

pub fn save_privacy(app: &AppHandle, p: Privacy) -> Result<(), String> {
    let file = privacy_file(app).ok_or("no config folder")?;
    if let Some(dir) = file.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    let json = serde_json::to_vec_pretty(&p).map_err(|e| e.to_string())?;
    std::fs::write(&file, json).map_err(|e| e.to_string())?;
    CONSENT.store(p.crash_reports, Ordering::Relaxed);
    Ok(())
}

/// Starts Sentry when the build has a DSN. Events are dropped unless the
/// scout agreed (checked per event, so the switch in Settings works at
/// once). Keep the guard for the app's life.
pub fn init_sentry() -> Option<sentry::ClientInitGuard> {
    let dsn = DSN?;
    let guard = sentry::init((
        dsn,
        sentry::ClientOptions {
            release: sentry::release_name!(),
            environment: Some(Cow::Borrowed(if cfg!(debug_assertions) {
                "development"
            } else {
                "production"
            })),
            send_default_pii: false,
            server_name: None,
            before_send: Some(Arc::new(|e| {
                CONSENT.load(Ordering::Relaxed).then(|| scrub(e, &home()))
            })),
            ..Default::default()
        },
    ));
    Some(guard)
}

/// `COURTSIDE_SENTRY_TEST=1`: sends one test event at startup, for this
/// run only (the saved answer isn't touched), to check a build's DSN.
pub fn send_test_event() {
    if std::env::var_os("COURTSIDE_SENTRY_TEST").is_none() {
        return;
    }
    if DSN.is_none() {
        log::warn!("sentry test: this build has no SENTRY_DSN");
        return;
    }
    CONSENT.store(true, Ordering::Relaxed);
    let id = sentry::capture_message("Courtside crash-report test", sentry::Level::Info);
    let sent = sentry::Hub::current()
        .client()
        .is_some_and(|c| c.flush(Some(std::time::Duration::from_secs(5))));
    log::info!("sentry test: event {id}, flushed {sent}");
}

/// Sets this computer's answer at startup.
pub fn set_consent(p: Privacy) {
    CONSENT.store(p.crash_reports, Ordering::Relaxed);
}

fn home() -> Option<String> {
    std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .ok()
        .filter(|h| h.len() > 1)
}

/// Takes the scout's folder names out of text: the home folder becomes `~`.
fn unhome(s: &str, home: &Option<String>) -> String {
    match home {
        Some(h) => s.replace(h.as_str(), "~"),
        None => s.to_owned(),
    }
}

/// What a crash report keeps: the error, where in the code, the app
/// version and the system. Not the user, the computer's name, extra data
/// or folder names.
fn scrub(
    mut e: sentry::protocol::Event<'static>,
    home: &Option<String>,
) -> sentry::protocol::Event<'static> {
    e.user = None;
    e.server_name = None;
    e.request = None;
    e.extra.clear();
    e.contexts
        .retain(|k, _| matches!(k.as_str(), "os" | "rust" | "device" | "app"));
    if let Some(sentry::protocol::Context::Device(d)) = e.contexts.get_mut("device") {
        d.name = None;
    }
    e.message = e.message.map(|m| unhome(&m, home));
    for x in &mut e.exception.values {
        x.value = x.value.as_deref().map(|v| unhome(v, home));
        if let Some(st) = &mut x.stacktrace {
            for f in &mut st.frames {
                f.abs_path = f.abs_path.as_deref().map(|p| unhome(p, home));
                f.filename = f.filename.as_deref().map(|p| unhome(p, home));
                f.vars.clear();
            }
        }
    }
    for b in &mut e.breadcrumbs.values {
        b.message = b.message.as_deref().map(|m| unhome(m, home));
        b.data.clear();
    }
    e
}

/// Writes panics to the log (Sentry's own hook, installed first, reports
/// them when the scout agreed).
pub fn log_panics() {
    let before = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        log::error!("panic: {info}");
        before(info);
    }));
}

/// An error in the window (JavaScript): logged, and reported with the
/// console actions before it when the scout agreed.
pub fn js_error(message: &str, stack: &str, actions: &[String]) {
    log::error!("window error: {message}\n{stack}");
    if !CONSENT.load(Ordering::Relaxed) {
        return;
    }
    sentry::with_scope(
        |scope| scope.set_tag("source", "window"),
        || {
            for a in actions {
                sentry::add_breadcrumb(sentry::Breadcrumb {
                    category: Some("action".into()),
                    message: Some(a.clone()),
                    ..Default::default()
                });
            }
            sentry::capture_message(&format!("{message}\n{stack}"), sentry::Level::Error)
        },
    );
}

/// The date and time in UTC, `YYYY-MM-DD HH.MM`, from Unix milliseconds.
fn stamp(ms: u64) -> String {
    let secs = ms / 1000;
    let (days, rest) = (secs / 86_400, secs % 86_400);
    // Days since 1970-01-01 to a civil date (Howard Hinnant's algorithm).
    let z = i64::try_from(days).unwrap_or(0) + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!(
        "{y:04}-{m:02}-{d:02} {:02}.{:02}",
        rest / 3600,
        (rest % 3600) / 60
    )
}

/// Percent-encodes text for a `mailto:` link.
fn encode(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(char::from(b));
            }
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

/// The last `n` lines of a text file.
fn tail(path: &Path, n: usize) -> Option<String> {
    let text = std::fs::read_to_string(path).ok()?;
    let lines: Vec<&str> = text.lines().collect();
    Some(lines[lines.len().saturating_sub(n)..].join("\n"))
}

/// What a report says about the open match: where it is and what Checks
/// found, by rule. No names or codes.
pub struct MatchSummary {
    pub set: u8,
    pub score: (u8, u8),
    pub codes: usize,
    /// "warning · rotation" → count.
    pub issues: BTreeMap<String, usize>,
}

pub struct Report<'a> {
    pub note: &'a str,
    pub actions: &'a [String],
    pub game: Option<MatchSummary>,
    pub crash_reports: bool,
    pub now_ms: u64,
}

/// Writes the report folder in `base` (Downloads): `report.txt` and the
/// last lines of the log in `log_dir`; `write_match` adds the match.
/// Returns the folder and the email's text.
pub fn write_report(
    base: &Path,
    log_dir: Option<&Path>,
    r: &Report<'_>,
    write_match: impl FnOnce(&Path) -> Result<(), String>,
    with_match: bool,
) -> Result<(PathBuf, String), String> {
    let when = stamp(r.now_ms);
    let folder = base.join(format!("Courtside report {when}"));
    std::fs::create_dir_all(&folder).map_err(|e| e.to_string())?;

    let mut text = format!(
        "Courtside {} ({})\nSystem: {} {}\nWhen: {when} UTC\nCrash reports: {}\n",
        env!("CARGO_PKG_VERSION"),
        if cfg!(debug_assertions) {
            "development"
        } else {
            "release"
        },
        std::env::consts::OS,
        std::env::consts::ARCH,
        if r.crash_reports { "on" } else { "off" },
    );
    text.push_str(&format!("\nWhat happened:\n{}\n", r.note.trim()));
    match &r.game {
        Some(g) => {
            text.push_str(&format!(
                "\nMatch: set {}, {}:{}, {} codes\n",
                g.set, g.score.0, g.score.1, g.codes
            ));
            if g.issues.is_empty() {
                text.push_str("Checks: nothing\n");
            } else {
                text.push_str("Checks:\n");
                for (k, n) in &g.issues {
                    text.push_str(&format!("  {k}: {n}\n"));
                }
            }
            text.push_str(if with_match {
                "The match is attached (match.dvw).\n"
            } else {
                "The match isn't attached.\n"
            });
        }
        None => text.push_str("\nNo match open.\n"),
    }
    text.push_str("\nLast actions, oldest first:\n");
    for a in r.actions {
        text.push_str(&format!("  {a}\n"));
    }
    std::fs::write(folder.join("report.txt"), &text).map_err(|e| e.to_string())?;

    if let Some(dir) = log_dir
        && let Some(log) = tail(&dir.join(format!("{LOG_NAME}.log")), 3000)
    {
        let log = unhome(&log, &home());
        std::fs::write(folder.join("courtside.log"), log).map_err(|e| e.to_string())?;
    }
    if with_match {
        write_match(&folder.join("match.dvw"))?;
    }
    Ok((folder, text))
}

/// The `mailto:` link for a report: the summary in the body, the files to
/// attach named.
pub fn mail_link(folder: &Path, text: &str) -> String {
    let name = folder
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let body = format!(
        "{text}\n---\nPlease attach the files in the folder \"{name}\" (it opened next to this email)."
    );
    format!(
        "mailto:{REPORT_TO}?subject={}&body={}",
        encode("Courtside problem report"),
        encode(&body)
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stamps_are_utc_dates() {
        assert_eq!(stamp(0), "1970-01-01 00.00");
        // 2026-10-24 14:32:10 UTC
        assert_eq!(stamp(1_792_852_330_000), "2026-10-24 14.32");
        assert_eq!(stamp(951_782_400_000), "2000-02-29 00.00");
    }

    #[test]
    fn mail_text_is_encoded() {
        assert_eq!(encode("a b&c=é"), "a%20b%26c%3D%C3%A9");
    }

    #[test]
    fn home_folders_come_out() {
        let home = Some("/Users/someone".to_owned());
        assert_eq!(
            unhome("/Users/someone/Documents/x.courtside", &home),
            "~/Documents/x.courtside"
        );
    }

    #[test]
    fn a_report_folder_has_the_summary_the_log_and_the_match_when_asked() {
        let dir =
            std::env::temp_dir().join(format!("courtside-report-test-{}", std::process::id()));
        let logs = dir.join("logs");
        std::fs::create_dir_all(&logs).unwrap();
        std::fs::write(logs.join("courtside.log"), "one\ntwo\n").unwrap();
        let mut issues = BTreeMap::new();
        issues.insert("warning · rotation".to_owned(), 2);
        let r = Report {
            note: "The point went to the wrong team.",
            actions: &["commit".to_owned(), "key:undo".to_owned()],
            game: Some(MatchSummary {
                set: 2,
                score: (14, 12),
                codes: 300,
                issues,
            }),
            crash_reports: false,
            now_ms: 1_792_852_330_000,
        };
        let (folder, text) = write_report(
            &dir,
            Some(&logs),
            &r,
            |p| std::fs::write(p, "[3DATAVOLLEYSCOUT]").map_err(|e| e.to_string()),
            true,
        )
        .unwrap();
        assert!(folder.ends_with("Courtside report 2026-10-24 14.32"));
        assert!(text.contains("set 2, 14:12, 300 codes"), "{text}");
        assert!(text.contains("warning · rotation: 2"));
        assert!(text.contains("  key:undo"));
        assert!(text.contains("The match is attached"));
        assert_eq!(
            std::fs::read_to_string(folder.join("courtside.log")).unwrap(),
            "one\ntwo"
        );
        assert!(folder.join("match.dvw").exists());
        let link = mail_link(&folder, &text);
        assert!(
            link.starts_with("mailto:") && link.contains("subject=Courtside%20problem%20report")
        );
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_crash_report_keeps_no_user_or_paths() {
        let mut e = sentry::protocol::Event {
            message: Some("can't open /Users/someone/season.courtside".into()),
            server_name: Some("Someones-MacBook".into()),
            user: Some(sentry::User {
                username: Some("someone".into()),
                ..Default::default()
            }),
            ..Default::default()
        };
        e.extra.insert("x".into(), "y".into());
        let e = scrub(e, &Some("/Users/someone".to_owned()));
        assert_eq!(e.message.as_deref(), Some("can't open ~/season.courtside"));
        assert!(e.server_name.is_none() && e.user.is_none() && e.extra.is_empty());
    }
}
