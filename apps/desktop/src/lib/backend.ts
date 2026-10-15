// The only way the UI reaches match data: Tauri commands into the Rust
// backend (docs/adr/0001). In a plain browser (`pnpm dev`) it serves the
// made-up fixture instead, so the layout can be worked on without Tauri.

import { invoke } from "@tauri-apps/api/core";
import { fixture } from "./fixture";
import type { Settings } from "./settings";
import type { MatchItem, Preview, Side, Snapshot } from "./snapshot";
import type { PlayerDetail, PlayerListItem, TeamDetail, TeamForm, TeamImportView } from "./teams";

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
export async function newMatch(
  home: string,
  visiting: string,
  date: string | null,
): Promise<Snapshot> {
  return invoke<Snapshot>("match_new", { home, visiting, date });
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

export async function preview(text: string): Promise<Preview> {
  if (!inTauri) return { codes: [], ok: false };
  return invoke<Preview>("preview", { text });
}

function clock(): number[] {
  const now = new Date();
  return [now.getHours(), now.getMinutes(), now.getSeconds()];
}

/** Adds the command line's codes, stamped with the local time of day. */
export async function commit(text: string): Promise<Snapshot> {
  if (!inTauri) throw new Error("Coding needs the desktop app.");
  return invoke<Snapshot>("commit", { text, clock: clock() });
}

/** Enters a team's six (zones 1–6) and setter for the set in progress. */
export async function setLineup(team: Side, court: number[], setter: number): Promise<Snapshot> {
  if (!inTauri) throw new Error("Coding needs the desktop app.");
  return invoke<Snapshot>("lineup_set", { team, court, setter, clock: clock() });
}

/** Starts a golden set once the match is over. */
export async function startGoldenSet(): Promise<Snapshot> {
  if (!inTauri) throw new Error("Coding needs the desktop app.");
  return invoke<Snapshot>("golden_set_start", { clock: clock() });
}

/** Corrects a team's six and setter for the set in progress. */
export async function correctLineup(
  team: Side,
  court: number[],
  setter: number,
): Promise<Snapshot> {
  if (!inTauri) throw new Error("Coding needs the desktop app.");
  return invoke<Snapshot>("lineup_correct", { team, court, setter, clock: clock() });
}

export async function exportDvw(path: string): Promise<void> {
  await invoke("export_dvw", { path });
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

export async function settings(): Promise<Settings | null> {
  if (!inTauri) return null;
  return invoke<Settings>("settings_get");
}

export async function saveSettings(settings: Settings): Promise<void> {
  await invoke("settings_save", { settings });
}
