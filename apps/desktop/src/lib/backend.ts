// The only way the UI reaches match data: Tauri commands into the Rust
// backend (docs/adr/0001). In a plain browser (`pnpm dev`) it serves the
// made-up fixture instead, so the layout can be worked on without Tauri.

import { invoke as tauriInvoke } from "@tauri-apps/api/core";
import { fixture, fixtureTeams } from "./fixture";
import type { Keys, Settings } from "./settings";
import type { ClipFilter, Clips, MatchItem, Preview, Side, Snapshot } from "./snapshot";
import type {
  PlayerDetail,
  PlayerListItem,
  TeamDetail,
  TeamForm,
  TeamImportView,
  VenueView,
} from "./teams";
import type { SheetCheck, SheetChoice, SheetPreview } from "./scoresheet";
import { noteAction } from "./trail";

// Polled or sent while typing: too many to say what the scout did.
const QUIET = new Set(["video_state", "video_place", "preview", "snapshot", "report_js_error"]);

/** A backend command, noted for problem reports by name. */
function invoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  if (!QUIET.has(cmd)) noteAction(cmd);
  return tauriInvoke<T>(cmd, args);
}

export const inTauri = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

export async function snapshot(): Promise<Snapshot | null> {
  if (!inTauri) return fixture();
  return invoke<Snapshot | null>("snapshot");
}

/** Reopens the season and match open last (after a restart or crash). */
export async function resume(): Promise<Snapshot | null> {
  if (!inTauri) return fixture();
  return invoke<Snapshot | null>("resume");
}

export async function teams(): Promise<TeamDetail[]> {
  if (!inTauri) return fixtureTeams();
  return invoke<TeamDetail[]>("teams");
}

/** Creates (`id` null) or updates a team together with its roster. */
export async function saveTeam(
  id: string | null,
  team: TeamForm,
  players: PlayerDetail[],
): Promise<TeamDetail> {
  return invoke<TeamDetail>("team_save", { id, team, players });
}

/** Imports a team file (`.sq` or `.vst`). */
export async function importTeam(path: string): Promise<TeamImportView> {
  return invoke<TeamImportView>("team_import", { path });
}

/** Writes a team file: `.vst` when the path ends so, else `.sq`. */
export async function exportTeam(id: string, path: string): Promise<void> {
  await invoke("team_export", { id, path });
}

export async function deleteTeam(id: string): Promise<void> {
  await invoke("team_delete", { id });
}

/** Folds team `from` into `into`; `pairs` are [from player id, into player id]. */
export async function mergeTeams(
  from: string,
  into: string,
  pairs: [string, string][],
): Promise<void> {
  await invoke("teams_merge", { from, into, pairs });
}

export async function players(): Promise<PlayerListItem[]> {
  return invoke<PlayerListItem[]>("players");
}

export async function mergePlayers(keep: string, drop: string): Promise<void> {
  await invoke("players_merge", { keep, drop });
}

/** The open match's own roster for one side. */
export async function matchRoster(home: boolean): Promise<PlayerDetail[]> {
  return invoke<PlayerDetail[]>("match_roster", { home });
}

export async function saveMatchRoster(home: boolean, players: PlayerDetail[]): Promise<Snapshot> {
  return invoke<Snapshot>("match_roster_save", { home, players });
}

/** Creates a match between two of the season's teams (by id) and opens it. */
/** A new match coded here; `venue` null takes the home team's. */
export async function newMatch(
  home: string,
  visiting: string,
  date: string | null,
  venue: VenueView | null,
): Promise<Snapshot> {
  return invoke<Snapshot>("match_new", { home, visiting, date, venue });
}

/** Where the open match is played. */
export async function matchVenue(): Promise<VenueView> {
  return invoke<VenueView>("match_venue");
}

export async function saveMatchVenue(venue: VenueView): Promise<void> {
  return invoke<void>("match_venue_save", { venue });
}

export async function createSeason(path: string, name: string): Promise<void> {
  await invoke("season_create", { path, name });
}

export async function openSeason(path: string): Promise<void> {
  await invoke("season_open", { path });
}

export async function matches(): Promise<MatchItem[]> {
  return invoke<MatchItem[]>("matches");
}

/** Imports a `.dvw` and opens it. */
export async function importMatch(path: string): Promise<Snapshot> {
  return invoke<Snapshot>("match_import", { path });
}

export async function openMatch(id: string): Promise<Snapshot> {
  return invoke<Snapshot>("match_open", { id });
}

/** Marks where a play went, in its own frame (court units); no start: its start zone; neither: clear. */
export async function setPoints(
  id: string,
  start: [number, number] | null,
  end: [number, number] | null,
): Promise<Snapshot> {
  return invoke<Snapshot>("points_set", { id, start, end });
}

/** Reads back the command line; with `before`, as codes going in just before that code. */
export async function preview(text: string, before: string | null = null): Promise<Preview> {
  if (!inTauri) return { codes: [], ok: false };
  return invoke<Preview>("preview", { text, before });
}

function clock(now = new Date()): number[] {
  return [now.getHours(), now.getMinutes(), now.getSeconds()];
}

/**
 * Adds the command line's codes, stamped with the local time of day the
 * line was started (`at`, else now) and the video time in ms at the first
 * keystroke of each code (`video`, one per space-separated code).
 */
export async function commit(
  text: string,
  video: (number | null)[] = [],
  at?: Date,
): Promise<Snapshot> {
  if (!inTauri) throw new Error("Coding needs the desktop app.");
  return invoke<Snapshot>("commit", { text, clock: clock(at), video });
}

/** Syncs the match to its video: the first serve is at `serveMs`. */
export async function videoSync(serveMs: number): Promise<Snapshot> {
  if (!inTauri) throw new Error("Syncing needs the desktop app.");
  return invoke<Snapshot>("video_sync", { serveMs: Math.round(serveMs) });
}

/** The open match's clip playlist for `filter`. */
export async function clips(filter: ClipFilter): Promise<Clips> {
  if (!inTauri) return { clips: [], withoutVideo: 0 };
  return invoke<Clips>("clips", { filter });
}

/**
 * The codes a search in the code log finds, by event id in match order.
 * In a browser, a text search over the fixture's rows.
 */
export async function findCodes(filter: ClipFilter): Promise<string[]> {
  if (!inTauri) {
    if (!filter.text) return [];
    return fixture()
      .groups.flatMap((g) => g.rows)
      .filter((r) => r.text.includes(filter.text))
      .map((r) => r.id);
  }
  return invoke<string[]>("find_codes", { filter });
}

/** Enters a team's six (zones 1–6) and setter for the set in progress. */
export async function setLineup(team: Side, court: number[], setter: number): Promise<Snapshot> {
  if (!inTauri) throw new Error("Coding needs the desktop app.");
  return invoke<Snapshot>("lineup_set", { team, court, setter, clock: clock() });
}

/** Ends the set in progress once its score wins it; `minutes`: its length as the scout gave it. */
export async function endSet(minutes?: number): Promise<Snapshot> {
  if (!inTauri) throw new Error("Coding needs the desktop app.");
  return invoke<Snapshot>("set_end", { clock: clock(), minutes: minutes ?? null });
}

/** Starts a golden set once the match is over. */
export async function startGoldenSet(): Promise<Snapshot> {
  if (!inTauri) throw new Error("Coding needs the desktop app.");
  return invoke<Snapshot>("golden_set_start", { clock: clock() });
}

/** Corrects a team's six and setter for a set; the rest replays from it. */
export async function correctLineup(
  team: Side,
  set: number,
  court: number[],
  setter: number,
): Promise<Snapshot> {
  if (!inTauri) throw new Error("Coding needs the desktop app.");
  return invoke<Snapshot>("lineup_correct", { team, set, court, setter, clock: clock() });
}

/** The video player's state, for the transport bar. */
export interface PlayerState {
  path: string | null;
  /** Seconds into the video. */
  time: number | null;
  duration: number | null;
  paused: boolean;
  speed: number;
  fps: number | null;
}

/** The panel's place in the window, in CSS pixels. */
export interface Bounds {
  x: number;
  y: number;
  width: number;
  height: number;
}

export async function videoOpen(path: string, bounds: Bounds): Promise<PlayerState> {
  return invoke<PlayerState>("video_open", { path, bounds });
}

export async function videoPlace(bounds: Bounds, visible: boolean): Promise<void> {
  if (!inTauri) return;
  await invoke("video_place", { bounds, visible });
}

/** An mpv command: `["seek", "-5", "relative+exact"]`, `["frame-step"]` … */
export async function videoCommand(args: string[]): Promise<PlayerState> {
  return invoke<PlayerState>("video_command", { args });
}

export async function videoPause(paused: boolean): Promise<PlayerState> {
  return invoke<PlayerState>("video_pause", { paused });
}

export async function videoSpeed(speed: number): Promise<PlayerState> {
  return invoke<PlayerState>("video_speed", { speed });
}

export async function videoState(): Promise<PlayerState | null> {
  if (!inTauri) return null;
  return invoke<PlayerState>("video_state");
}

/** Writes the match as a .dvw; the video's path goes in only `withVideo`. */
export async function exportDvw(path: string, withVideo = false): Promise<void> {
  await invoke("export_dvw", { path, withVideo });
}

/** Remembers the video opened with the match. */
export async function matchVideoSet(path: string | null): Promise<Snapshot> {
  return invoke<Snapshot>("match_video_set", { path });
}

function needsApp(): void {
  if (!inTauri) throw new Error("Coding needs the desktop app.");
}

export async function undo(): Promise<Snapshot> {
  needsApp();
  return invoke<Snapshot>("undo");
}

export async function redo(): Promise<Snapshot> {
  needsApp();
  return invoke<Snapshot>("redo");
}

/** Replaces an earlier code (by event id) with `text`. */
export async function correct(id: string, text: string): Promise<Snapshot> {
  needsApp();
  return invoke<Snapshot>("code_correct", { id, text, clock: clock() });
}

/** Adds `text` just before an earlier code. */
export async function insertBefore(id: string, text: string): Promise<Snapshot> {
  needsApp();
  return invoke<Snapshot>("code_insert", { id, text, clock: clock() });
}

export async function remove(id: string): Promise<Snapshot> {
  needsApp();
  return invoke<Snapshot>("code_remove", { id });
}

/** Writes the console's rotations again from the rally at `id` to the set's end. */
export async function fixRotations(id: string): Promise<Snapshot> {
  needsApp();
  return invoke<Snapshot>("rotations_fix", { id });
}

/** What "Fix rotations from here" would change; nothing is changed. */
export interface FixPreview {
  codes: number;
  fromRally: number | null;
  toRally: number | null;
  issuesBefore: number;
  issuesAfter: number;
}

export async function fixRotationsPreview(id: string): Promise<FixPreview> {
  needsApp();
  return invoke<FixPreview>("rotations_fix_preview", { id });
}

export async function settings(): Promise<Settings | null> {
  if (!inTauri) return null;
  return invoke<Settings>("settings_get");
}

export async function saveSettings(settings: Settings): Promise<void> {
  await invoke("settings_save", { settings });
}

/** This computer's keys. */
export async function keys(): Promise<Keys | null> {
  if (!inTauri) return null;
  return invoke<Keys>("keys_get");
}

export async function saveKeys(keys: Keys): Promise<void> {
  await invoke("keys_save", { keys });
}

/** This computer's choice about crash reports; `available`: the build can send them. */
export interface Privacy {
  crashReports: boolean;
  asked: boolean;
  available: boolean;
}

export async function privacy(): Promise<Privacy> {
  if (!inTauri) return { crashReports: false, asked: true, available: false };
  return invoke<Privacy>("privacy_get");
}

export async function setPrivacy(crashReports: boolean): Promise<Privacy> {
  return invoke<Privacy>("privacy_set", { crashReports });
}

export async function reportJsError(
  message: string,
  stack: string,
  actions: string[],
): Promise<void> {
  if (!inTauri) return;
  return invoke("report_js_error", { message, stack, actions });
}

/**
 * Writes a problem report folder (with the match when `includeMatch`),
 * shows it and opens an email; the folder's path.
 */
export async function reportProblem(
  note: string,
  includeMatch: boolean,
  actions: string[],
): Promise<string> {
  if (!inTauri) throw new Error("Reports need the desktop app.");
  return invoke<string>("report_problem", { note, includeMatch, actions });
}

/** Reads a scoresheet PDF and matches it to the season, for review. */
export async function readScoresheet(path: string): Promise<SheetPreview> {
  return invoke<SheetPreview>("scoresheet_read", { path });
}

/** Creates a match from a scoresheet and opens it. */
export async function matchFromScoresheet(path: string, choice: SheetChoice): Promise<Snapshot> {
  return invoke<Snapshot>("match_from_scoresheet", { path, choice });
}

/** Attaches a scoresheet to the open match. */
export async function attachScoresheet(path: string): Promise<Snapshot> {
  return invoke<Snapshot>("scoresheet_attach", { path });
}

export async function removeScoresheet(): Promise<Snapshot> {
  return invoke<Snapshot>("scoresheet_remove");
}

/** Where the open match and its scoresheet disagree; null without one. */
export async function scoresheetCheck(): Promise<SheetCheck | null> {
  return invoke<SheetCheck | null>("scoresheet_check");
}
