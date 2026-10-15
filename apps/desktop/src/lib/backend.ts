// The only way the UI reaches match data: Tauri commands into the Rust
// backend (docs/adr/0001). In a plain browser (`pnpm dev`) it serves the
// made-up fixture instead, so the layout can be worked on without Tauri.

import { invoke } from "@tauri-apps/api/core";
import { fixture } from "./fixture";
import type { MatchItem, Preview, Side, Snapshot } from "./snapshot";

export const inTauri = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

export async function snapshot(): Promise<Snapshot | null> {
  if (!inTauri) return fixture();
  return invoke<Snapshot | null>("snapshot");
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

export async function importMatch(path: string): Promise<MatchItem[]> {
  return invoke<MatchItem[]>("match_import", { path });
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
