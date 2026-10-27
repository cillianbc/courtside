//! Courtside desktop: Tauri commands over a [`Session`] (docs/adr/0001).
//! Each command returns what the UI needs next; errors come back as text.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::path::PathBuf;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use courtside_console::{
    ClipFilter, Clips, ClockTime, FixPreview, Keys, MatchId, MatchItem, PlayerDetail,
    PlayerListItem, Preview, Resume, Session, Settings, Snapshot, Team, TeamDetail, TeamForm,
    TeamImportView, VenueView,
};
use tauri::{AppHandle, Manager, State};
use tauri_plugin_opener::OpenerExt;

mod diagnostics;
mod video;

type Video = Mutex<video::Player>;

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
        log::warn!("can't write {}: {e}", file.display());
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
fn settings_get(state: State<'_, Shared>) -> Reply<Settings> {
    with(&state, |s| s.settings())
}

#[tauri::command]
fn settings_save(state: State<'_, Shared>, settings: Settings) -> Reply<()> {
    with(&state, |s| s.save_settings(&settings))
}

/// Where this computer's keys live: the app's config directory.
fn keys_file(app: &AppHandle) -> Reply<PathBuf> {
    app.path()
        .app_config_dir()
        .map(|d| d.join("keys.json"))
        .map_err(|e| e.to_string())
}

/// This computer's keys. Without a keys file, the open season's keys
/// (where they were kept before) become this computer's.
#[tauri::command]
fn keys_get(app: AppHandle, state: State<'_, Shared>) -> Reply<Keys> {
    let file = keys_file(&app)?;
    if let Some(keys) = Keys::load(&file) {
        return Ok(keys);
    }
    let Ok(settings) = with(&state, |s| s.settings()) else {
        return Ok(Keys::default());
    };
    let keys = Keys::from_season(&settings);
    if let Err(e) = keys.save(&file) {
        log::warn!("{e}");
    }
    Ok(keys)
}

#[tauri::command]
fn keys_save(app: AppHandle, keys: Keys) -> Reply<()> {
    keys.save(&keys_file(&app)?).map_err(|e| e.to_string())
}

/// This computer's choice about crash reports, and whether this build can
/// send them.
#[tauri::command]
fn privacy_get(app: AppHandle) -> diagnostics::PrivacyView {
    diagnostics::view(diagnostics::load_privacy(&app))
}

#[tauri::command]
fn privacy_set(app: AppHandle, crash_reports: bool) -> Reply<diagnostics::PrivacyView> {
    let p = diagnostics::Privacy {
        crash_reports,
        asked: true,
    };
    diagnostics::save_privacy(&app, p)?;
    Ok(diagnostics::view(p))
}

/// An error in the window: logged, and reported when the scout agreed.
/// `actions`: the console actions before it, oldest first.
#[tauri::command]
fn report_js_error(message: String, stack: String, actions: Vec<String>) {
    diagnostics::js_error(&message, &stack, &actions);
}

/// "Report a problem": writes the report folder (with the match when
/// `include_match`), shows it, and opens an email to us. Returns the
/// folder.
#[tauri::command]
fn report_problem(
    app: AppHandle,
    state: State<'_, Shared>,
    note: String,
    include_match: bool,
    actions: Vec<String>,
) -> Reply<String> {
    let game = with(&state, |s| s.snapshot()).ok().map(|snap| {
        let mut issues = std::collections::BTreeMap::new();
        for i in &snap.issues {
            *issues
                .entry(format!("{} · {}", i.severity, i.rule))
                .or_insert(0) += 1;
        }
        diagnostics::MatchSummary {
            set: snap.set,
            score: (snap.score.home, snap.score.visiting),
            codes: snap.groups.iter().map(|g| g.rows.len()).sum(),
            issues,
        }
    });
    let with_match = include_match && game.is_some();
    let report = diagnostics::Report {
        note: &note,
        actions: &actions,
        game,
        crash_reports: diagnostics::load_privacy(&app).crash_reports,
        now_ms: now_ms(),
    };
    let base = app
        .path()
        .download_dir()
        .or_else(|_| app.path().home_dir())
        .map_err(|e| e.to_string())?;
    let logs = app.path().app_log_dir().ok();
    let (folder, text) = diagnostics::write_report(
        &base,
        logs.as_deref(),
        &report,
        |path| with(&state, |s| s.export_dvw(path, false)),
        with_match,
    )?;
    log::info!("problem report written");
    let opener = app.opener();
    if let Err(e) = opener.reveal_item_in_dir(folder.join("report.txt")) {
        log::warn!("can't show the report folder: {e}");
    }
    opener
        .open_url(diagnostics::mail_link(&folder, &text), None::<&str>)
        .map_err(|e| {
            format!(
                "The report is in {}, but the mail app didn't open: {e}",
                folder.display()
            )
        })?;
    Ok(folder.display().to_string())
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
/// `venue` `None` takes the home team's.
#[tauri::command]
fn match_new(
    app: AppHandle,
    state: State<'_, Shared>,
    home: String,
    visiting: String,
    date: Option<String>,
    venue: Option<VenueView>,
) -> Reply<Snapshot> {
    let snap = with(&state, |s| {
        s.new_match(&home, &visiting, date.as_deref(), venue.as_ref(), now_ms())
    })?;
    remember(&app, &state);
    Ok(snap)
}

/// Where the open match is played.
#[tauri::command]
fn match_venue(state: State<'_, Shared>) -> Reply<VenueView> {
    with(&state, |s| s.match_venue())
}

#[tauri::command]
fn match_venue_save(state: State<'_, Shared>, venue: VenueView) -> Reply<()> {
    with(&state, |s| s.set_match_venue(&venue))
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

/// Imports a `.dvw` file and opens it, as the scout imports a file to
/// work on it.
#[tauri::command]
fn match_import(app: AppHandle, state: State<'_, Shared>, path: PathBuf) -> Reply<Snapshot> {
    let snap = with(&state, |s| {
        let id = s.import_file(&path, now_ms())?;
        s.open_match(id, now_ms())
    })?;
    remember(&app, &state);
    Ok(snap)
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

/// Marks where a play went on the court, in the play's own frame: `[x, y]`
/// in court units; no start means its start zone, neither clears.
#[tauri::command]
fn points_set(
    state: State<'_, Shared>,
    id: String,
    start: Option<(f64, f64)>,
    end: Option<(f64, f64)>,
) -> Reply<Snapshot> {
    with(&state, |s| s.set_points(&id, start, end))
}

/// Reads back the command line; with `before`, as codes going in just
/// before that earlier code.
#[tauri::command]
fn preview(state: State<'_, Shared>, text: String, before: Option<String>) -> Reply<Preview> {
    with(&state, |s| match &before {
        Some(id) => s.preview_before(&text, id),
        None => s.preview(&text),
    })
}

/// Adds the command line's codes; `clock` is the local time of day
/// (hour, minute, second) for the `.dvw` time column, `video` the video
/// time in ms at the first keystroke of each code (none with no video).
#[tauri::command]
fn commit(
    state: State<'_, Shared>,
    text: String,
    clock: Option<[u8; 3]>,
    video: Option<Vec<Option<u64>>>,
) -> Reply<Snapshot> {
    let video = video.unwrap_or_default();
    with(&state, |s| s.commit_at(&text, clock_time(clock), &video))
}

/// Remembers the video opened with the match.
#[tauri::command]
fn match_video_set(state: State<'_, Shared>, path: Option<String>) -> Reply<Snapshot> {
    with(&state, |s| s.set_video(path.as_deref()))
}

/// Syncs the match to its video: the first serve is at `serve_ms`.
#[tauri::command]
fn video_sync(state: State<'_, Shared>, serve_ms: u64) -> Reply<Snapshot> {
    with(&state, |s| s.sync_video(serve_ms))
}

/// The open match's clip playlist for `filter`.
#[tauri::command]
fn clips(state: State<'_, Shared>, filter: ClipFilter) -> Reply<Clips> {
    with(&state, |s| s.clips(&filter))
}

/// The codes a search in the code log finds, by event id.
#[tauri::command]
fn find_codes(state: State<'_, Shared>, filter: ClipFilter) -> Reply<Vec<String>> {
    with(&state, |s| s.find_codes(&filter))
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
    let team = side(&team)?;
    with(&state, |s| {
        s.set_lineup(team, court, setter, clock_time(clock))
    })
}

/// Ends the set in progress once its score wins it (when set ends wait for
/// the scout), with the set's length in minutes when the scout gave it.
#[tauri::command]
fn set_end(
    state: State<'_, Shared>,
    clock: Option<[u8; 3]>,
    minutes: Option<u16>,
) -> Reply<Snapshot> {
    with(&state, |s| s.end_set(clock_time(clock), minutes))
}

/// Starts a golden set after the match is over.
#[tauri::command]
fn golden_set_start(state: State<'_, Shared>, clock: Option<[u8; 3]>) -> Reply<Snapshot> {
    with(&state, |s| s.start_golden_set(clock_time(clock)))
}

/// Corrects a team's six and setter for a set; the rest of the match
/// replays from it.
#[tauri::command]
fn lineup_correct(
    state: State<'_, Shared>,
    team: String,
    set: u8,
    court: [u8; 6],
    setter: u8,
    clock: Option<[u8; 3]>,
) -> Reply<Snapshot> {
    let team = side(&team)?;
    with(&state, |s| {
        s.correct_lineup(team, set, court, setter, clock_time(clock))
    })
}

fn side(team: &str) -> Result<Team, String> {
    match team {
        "home" => Ok(Team::Home),
        "visiting" => Ok(Team::Visiting),
        other => Err(format!("unknown team {other:?}")),
    }
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
fn rotations_fix(state: State<'_, Shared>, id: String) -> Reply<Snapshot> {
    with(&state, |s| s.fix_rotations(&id))
}

/// What "Fix rotations from here" would change, without changing it.
#[tauri::command]
fn rotations_fix_preview(state: State<'_, Shared>, id: String) -> Reply<FixPreview> {
    with(&state, |s| s.fix_rotations_preview(&id))
}

#[tauri::command]
fn export_dvw(state: State<'_, Shared>, path: PathBuf, with_video: Option<bool>) -> Reply<()> {
    with(&state, |s| s.export_dvw(&path, with_video.unwrap_or(false)))
}

/// The open match's state, or `None` when no match is open.
#[tauri::command]
fn snapshot(state: State<'_, Shared>) -> Reply<Option<Snapshot>> {
    with(&state, |s| Ok(s.snapshot().ok()))
}

fn with_video<T>(
    state: &State<'_, Video>,
    f: impl FnOnce(&mut video::Player) -> Result<T, String>,
) -> Reply<T> {
    let mut player = state.lock().map_err(|_| "video lock poisoned".to_owned())?;
    f(&mut player)
}

// Video commands are synchronous, so Tauri runs them on the main thread,
// where AppKit wants the player's view touched.

/// Opens a video file in the panel at `bounds`.
#[tauri::command]
fn video_open(
    window: tauri::WebviewWindow,
    state: State<'_, Video>,
    path: String,
    bounds: video::Bounds,
) -> Reply<video::PlayerState> {
    with_video(&state, |p| {
        p.open(&window, &path, bounds)?;
        Ok(p.state())
    })
}

/// Keeps the video on the panel as the layout changes.
#[tauri::command]
fn video_place(state: State<'_, Video>, bounds: video::Bounds, visible: bool) -> Reply<()> {
    with_video(&state, |p| p.place(bounds, visible))
}

/// An mpv command (`seek`, `frame-step`, `cycle pause` …).
#[tauri::command]
fn video_command(state: State<'_, Video>, args: Vec<String>) -> Reply<video::PlayerState> {
    with_video(&state, |p| {
        p.command(&args)?;
        Ok(p.state())
    })
}

#[tauri::command]
fn video_speed(state: State<'_, Video>, speed: f64) -> Reply<video::PlayerState> {
    with_video(&state, |p| {
        p.set_speed(speed)?;
        Ok(p.state())
    })
}

#[tauri::command]
fn video_pause(state: State<'_, Video>, paused: bool) -> Reply<video::PlayerState> {
    with_video(&state, |p| {
        p.set_paused(paused)?;
        Ok(p.state())
    })
}

#[tauri::command]
fn video_state(state: State<'_, Video>) -> Reply<video::PlayerState> {
    with_video(&state, |p| Ok(p.state()))
}

#[tauri::command]
fn video_close(state: State<'_, Video>) -> Reply<()> {
    with_video(&state, video::Player::close)
}

/// Development check that playback works without clicking through the UI:
/// with `COURTSIDE_VIDEO_SMOKE=<file>` the app opens it over the top left of
/// the window, plays two seconds and prints what mpv reports.
fn video_smoke(app: &AppHandle) {
    let Ok(path) = std::env::var("COURTSIDE_VIDEO_SMOKE") else {
        return;
    };
    let app = app.clone();
    std::thread::spawn(move || {
        let step = |what: &'static str, f: fn(&AppHandle, &str)| {
            let (app2, path2) = (app.clone(), path.clone());
            let _ = app.run_on_main_thread(move || f(&app2, &path2));
            log::info!("video smoke: {what}");
            std::thread::sleep(std::time::Duration::from_secs(2));
        };
        std::thread::sleep(std::time::Duration::from_secs(3));
        step("open", |app, path| {
            let (Some(window), Some(state)) =
                (app.get_webview_window("main"), app.try_state::<Video>())
            else {
                return;
            };
            let bounds = video::Bounds {
                x: 24.0,
                y: 90.0,
                width: 640.0,
                height: 360.0,
            };
            let opened = state.lock().map_err(|e| e.to_string()).and_then(|mut p| {
                p.open(&window, path, bounds)?;
                p.set_paused(false)
            });
            log::info!("video smoke: opened {opened:?}");
        });
        step("state", |app, _| {
            if let Some(state) = app.try_state::<Video>()
                && let Ok(p) = state.lock()
            {
                log::info!("video smoke: {:?}", p.state());
            }
        });
        // A taller box, as the Video layout gives: the picture must follow
        // (no "viewport …, layer …" line after this one).
        step("resize", |app, _| {
            if let Some(state) = app.try_state::<Video>()
                && let Ok(mut p) = state.lock()
            {
                let bounds = video::Bounds {
                    x: 24.0,
                    y: 90.0,
                    width: 600.0,
                    height: 450.0,
                };
                log::info!("video smoke: resized {:?}", p.place(bounds, true));
            }
        });
        step("state", |app, _| {
            if let Some(state) = app.try_state::<Video>()
                && let Ok(p) = state.lock()
            {
                log::info!("video smoke: {:?}", p.state());
            }
        });
        // Closing frees mpv's render context before mpv; opening again
        // makes both anew.
        step("close", |app, _| {
            if let Some(state) = app.try_state::<Video>()
                && let Ok(mut p) = state.lock()
            {
                log::info!("video smoke: closed {:?}", p.close());
            }
        });
        step("reopen", |app, path| {
            let (Some(window), Some(state)) =
                (app.get_webview_window("main"), app.try_state::<Video>())
            else {
                return;
            };
            let bounds = video::Bounds {
                x: 24.0,
                y: 90.0,
                width: 640.0,
                height: 360.0,
            };
            let opened = state.lock().map_err(|e| e.to_string()).and_then(|mut p| {
                p.open(&window, path, bounds)?;
                p.set_paused(false)
            });
            log::info!("video smoke: reopened {opened:?}");
        });
        step("state", |app, _| {
            if let Some(state) = app.try_state::<Video>()
                && let Ok(p) = state.lock()
            {
                log::info!("video smoke: {:?}", p.state());
            }
        });
    });
}

fn main() {
    // Sentry's panic hook first, then the log's (diagnostics.rs).
    let _sentry = diagnostics::init_sentry();
    diagnostics::log_panics();
    let log = tauri_plugin_log::Builder::new()
        .targets([
            tauri_plugin_log::Target::new(tauri_plugin_log::TargetKind::LogDir {
                file_name: Some(diagnostics::LOG_NAME.into()),
            }),
            tauri_plugin_log::Target::new(tauri_plugin_log::TargetKind::Stderr),
        ])
        .level(log::LevelFilter::Info)
        .max_file_size(2_000_000)
        .rotation_strategy(tauri_plugin_log::RotationStrategy::KeepOne)
        .build();
    let result = tauri::Builder::default()
        .plugin(log)
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(Shared::new(Session::new()))
        .manage(Video::default())
        .setup(|app| {
            diagnostics::set_consent(diagnostics::load_privacy(app.handle()));
            diagnostics::send_test_event();
            log::info!("Courtside {} starting", env!("CARGO_PKG_VERSION"));
            video_smoke(app.handle());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            season_create,
            season_open,
            teams,
            team_save,
            team_delete,
            team_import,
            settings_get,
            settings_save,
            keys_get,
            keys_save,
            team_export,
            teams_merge,
            players,
            players_merge,
            match_roster,
            match_roster_save,
            match_new,
            match_venue,
            match_venue_save,
            resume,
            matches,
            match_import,
            match_open,
            snapshot,
            preview,
            points_set,
            commit,
            lineup_set,
            lineup_correct,
            golden_set_start,
            set_end,
            undo,
            redo,
            code_correct,
            code_insert,
            code_remove,
            rotations_fix,
            rotations_fix_preview,
            export_dvw,
            video_open,
            video_sync,
            clips,
            find_codes,
            privacy_get,
            privacy_set,
            report_js_error,
            report_problem,
            match_video_set,
            video_place,
            video_command,
            video_speed,
            video_pause,
            video_state,
            video_close
        ])
        .run(tauri::generate_context!());
    if let Err(e) = result {
        eprintln!("Courtside failed to start: {e}");
        std::process::exit(1);
    }
}
