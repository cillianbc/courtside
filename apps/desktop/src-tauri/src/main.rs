//! Courtside desktop: Tauri commands over a [`Session`] (docs/adr/0001).
//! Each command returns what the UI needs next; errors come back as text.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::path::PathBuf;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use courtside_console::{
    ClockTime, MatchId, MatchItem, PlayerDetail, PlayerListItem, Preview, Resume, Session,
    Snapshot, Team, TeamDetail, TeamForm, TeamImportView,
};
use tauri::{AppHandle, Manager, State};

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

/// Where the resume point lives: the app's config directory.
fn resume_file(app: &AppHandle) -> Option<PathBuf> {
    app.path()
        .app_config_dir()
        .ok()
        .map(|d| d.join("resume.json"))
}

/// Records the open season and match, so a restart reopens them. Failing
/// to write it doesn't fail the command: the match itself is saved.
fn remember(app: &AppHandle, state: &State<'_, Shared>) {
    let point = match state.lock() {
        Ok(s) => s.resume_point(),
        Err(_) => return,
    };
    if let Some(file) = resume_file(app)
        && let Err(e) = point.save(&file)
    {
        eprintln!("can't write {}: {e}", file.display());
    }
}

fn clock_time(clock: Option<[u8; 3]>) -> Option<ClockTime> {
    clock.map(|[hour, minute, second]| ClockTime {
        hour,
        minute,
        second,
    })
}

#[tauri::command]
fn season_create(
    app: AppHandle,
    state: State<'_, Shared>,
    path: PathBuf,
    name: String,
) -> Reply<()> {
    with(&state, |s| s.create_season(&path, &name, now_ms()))?;
    remember(&app, &state);
    Ok(())
}

#[tauri::command]
fn season_open(app: AppHandle, state: State<'_, Shared>, path: PathBuf) -> Reply<()> {
    with(&state, |s| s.open_season(&path))?;
    remember(&app, &state);
    Ok(())
}

#[tauri::command]
fn teams(state: State<'_, Shared>) -> Reply<Vec<TeamDetail>> {
    with(&state, |s| s.teams())
}

/// Creates (no `id`) or updates a team together with its roster.
#[tauri::command]
fn team_save(
    state: State<'_, Shared>,
    id: Option<String>,
    team: TeamForm,
    players: Vec<PlayerDetail>,
) -> Reply<TeamDetail> {
    with(&state, |s| {
        s.save_team(id.as_deref(), &team, &players, now_ms())
    })
}

/// Imports a team file (`.sq` or `.vst`).
#[tauri::command]
fn team_import(state: State<'_, Shared>, path: PathBuf) -> Reply<TeamImportView> {
    with(&state, |s| s.import_team_file(&path, now_ms()))
}

/// Writes a team file (`.sq`, or `.vst` by the path's extension).
#[tauri::command]
fn team_export(state: State<'_, Shared>, id: String, path: PathBuf) -> Reply<()> {
    with(&state, |s| s.export_team_file(&id, &path))
}

#[tauri::command]
fn team_delete(state: State<'_, Shared>, id: String) -> Reply<()> {
    with(&state, |s| s.delete_team(&id))
}

/// Folds team `from` into `into`; `pairs` are (from player, into player).
#[tauri::command]
fn teams_merge(
    state: State<'_, Shared>,
    from: String,
    into: String,
    pairs: Vec<(String, String)>,
) -> Reply<()> {
    with(&state, |s| s.merge_teams(&from, &into, &pairs))
}

#[tauri::command]
fn players(state: State<'_, Shared>) -> Reply<Vec<PlayerListItem>> {
    with(&state, |s| s.players())
}

#[tauri::command]
fn players_merge(state: State<'_, Shared>, keep: String, drop: String) -> Reply<()> {
    with(&state, |s| s.merge_players(&keep, &drop))
}

/// The open match's own roster for one side.
#[tauri::command]
fn match_roster(state: State<'_, Shared>, home: bool) -> Reply<Vec<PlayerDetail>> {
    with(&state, |s| s.match_roster(home))
}

#[tauri::command]
fn match_roster_save(
    state: State<'_, Shared>,
    home: bool,
    players: Vec<PlayerDetail>,
) -> Reply<Snapshot> {
    with(&state, |s| s.save_match_roster(home, &players, now_ms()))
}

/// Creates a match between two of the season's teams and opens it.
#[tauri::command]
fn match_new(
    app: AppHandle,
    state: State<'_, Shared>,
    home: String,
    visiting: String,
    date: Option<String>,
) -> Reply<Snapshot> {
    let snap = with(&state, |s| {
        s.new_match(&home, &visiting, date.as_deref(), now_ms())
    })?;
    remember(&app, &state);
    Ok(snap)
}

/// Reopens the season and match open last, after a restart or crash.
/// `None` when there is no match to go back to.
#[tauri::command]
fn resume(app: AppHandle, state: State<'_, Shared>) -> Reply<Option<Snapshot>> {
    let Some(file) = resume_file(&app) else {
        return Ok(None);
    };
    let point = Resume::load(&file);
    with(&state, |s| s.resume(&point, now_ms()))
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
fn match_open(app: AppHandle, state: State<'_, Shared>, id: String) -> Reply<Snapshot> {
    let id: MatchId = id
        .parse()
        .map_err(|e: courtside_console::StoreError| e.to_string())?;
    let snap = with(&state, |s| s.open_match(id, now_ms()))?;
    remember(&app, &state);
    Ok(snap)
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
fn undo(state: State<'_, Shared>) -> Reply<Snapshot> {
    with(&state, |s| s.undo())
}

#[tauri::command]
fn redo(state: State<'_, Shared>) -> Reply<Snapshot> {
    with(&state, |s| s.redo())
}

/// Replaces the code with event id `id` by `text`.
#[tauri::command]
fn code_correct(
    state: State<'_, Shared>,
    id: String,
    text: String,
    clock: Option<[u8; 3]>,
) -> Reply<Snapshot> {
    with(&state, |s| s.correct(&id, &text, clock_time(clock)))
}

/// Adds `text` before the code with event id `id`.
#[tauri::command]
fn code_insert(
    state: State<'_, Shared>,
    id: String,
    text: String,
    clock: Option<[u8; 3]>,
) -> Reply<Snapshot> {
    with(&state, |s| s.insert_before(&id, &text, clock_time(clock)))
}

#[tauri::command]
fn code_remove(state: State<'_, Shared>, id: String) -> Reply<Snapshot> {
    with(&state, |s| s.remove(&id))
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
            teams,
            team_save,
            team_delete,
            team_import,
            team_export,
            teams_merge,
            players,
            players_merge,
            match_roster,
            match_roster_save,
            match_new,
            resume,
            matches,
            match_import,
            match_open,
            snapshot,
            preview,
            commit,
            lineup_set,
            undo,
            redo,
            code_correct,
            code_insert,
            code_remove,
            export_dvw
        ])
        .run(tauri::generate_context!());
    if let Err(e) = result {
        eprintln!("Courtside failed to start: {e}");
        std::process::exit(1);
    }
}
