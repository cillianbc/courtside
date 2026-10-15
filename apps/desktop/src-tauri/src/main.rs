//! Courtside desktop: Tauri commands over a [`Session`] (docs/adr/0001).
//! Each command returns what the UI needs next; errors come back as text.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::path::PathBuf;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use courtside_console::{ClockTime, MatchId, MatchItem, Preview, Session, Snapshot, Team};
use tauri::State;

type Shared = Mutex<Session>;
type Reply<T> = Result<T, String>;

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| u64::try_from(d.as_millis()).unwrap_or(u64::MAX))
        .unwrap_or(0)
}

fn with<T>(
    state: &State<'_, Shared>,
    f: impl FnOnce(&mut Session) -> courtside_console::Result<T>,
) -> Reply<T> {
    let mut session = state
        .lock()
        .map_err(|_| "session lock poisoned".to_owned())?;
    f(&mut session).map_err(|e| e.to_string())
}

fn clock_time(clock: Option<[u8; 3]>) -> Option<ClockTime> {
    clock.map(|[hour, minute, second]| ClockTime {
        hour,
        minute,
        second,
    })
}

#[tauri::command]
fn season_create(state: State<'_, Shared>, path: PathBuf, name: String) -> Reply<()> {
    with(&state, |s| s.create_season(&path, &name, now_ms()))
}

#[tauri::command]
fn season_open(state: State<'_, Shared>, path: PathBuf) -> Reply<()> {
    with(&state, |s| s.open_season(&path))
}

#[tauri::command]
fn matches(state: State<'_, Shared>) -> Reply<Vec<MatchItem>> {
    with(&state, |s| s.matches())
}

/// Imports a `.dvw` file and returns the season's matches.
#[tauri::command]
fn match_import(state: State<'_, Shared>, path: PathBuf) -> Reply<Vec<MatchItem>> {
    with(&state, |s| {
        s.import_file(&path, now_ms())?;
        s.matches()
    })
}

#[tauri::command]
fn match_open(state: State<'_, Shared>, id: String) -> Reply<Snapshot> {
    let id: MatchId = id
        .parse()
        .map_err(|e: courtside_console::StoreError| e.to_string())?;
    with(&state, |s| s.open_match(id, now_ms()))
}

/// Reads back the command line.
#[tauri::command]
fn preview(state: State<'_, Shared>, text: String) -> Reply<Preview> {
    with(&state, |s| s.preview(&text))
}

/// Adds the command line's codes; `clock` is the local time of day
/// (hour, minute, second) for the `.dvw` time column.
#[tauri::command]
fn commit(state: State<'_, Shared>, text: String, clock: Option<[u8; 3]>) -> Reply<Snapshot> {
    with(&state, |s| s.commit(&text, clock_time(clock)))
}

/// Enters a team's six for the set in progress: `team` is `"home"` or
/// `"visiting"`, `court` shirt numbers in zones 1–6.
#[tauri::command]
fn lineup_set(
    state: State<'_, Shared>,
    team: String,
    court: [u8; 6],
    setter: u8,
    clock: Option<[u8; 3]>,
) -> Reply<Snapshot> {
    let team = match team.as_str() {
        "home" => Team::Home,
        "visiting" => Team::Visiting,
        other => return Err(format!("unknown team {other:?}")),
    };
    with(&state, |s| {
        s.set_lineup(team, court, setter, clock_time(clock))
    })
}

#[tauri::command]
fn export_dvw(state: State<'_, Shared>, path: PathBuf) -> Reply<()> {
    with(&state, |s| s.export_dvw(&path))
}

/// The open match's state, or `None` when no match is open.
#[tauri::command]
fn snapshot(state: State<'_, Shared>) -> Reply<Option<Snapshot>> {
    with(&state, |s| Ok(s.snapshot().ok()))
}

fn main() {
    let result = tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(Shared::new(Session::new()))
        .invoke_handler(tauri::generate_handler![
            season_create,
            season_open,
            matches,
            match_import,
            match_open,
            snapshot,
            preview,
            commit,
            lineup_set,
            export_dvw
        ])
        .run(tauri::generate_context!());
    if let Err(e) = result {
        eprintln!("Courtside failed to start: {e}");
        std::process::exit(1);
    }
}
