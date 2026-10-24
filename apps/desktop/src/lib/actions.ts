// What each keymap action does. Buttons on screen run the same code.

import { ask, save } from "@tauri-apps/plugin-dialog";
import * as backend from "./backend";
import { rotationCode, timeoutCode } from "./codes";
import { stepMatch } from "./filter";
import { pickCode } from "./jump";
import type { Side } from "./snapshot";
import { ui } from "./ui.svelte";
import { JUMP, LONG_JUMP, video } from "./video.svelte";

export async function exportDvw(): Promise<void> {
  const snap = ui.snapshot;
  if (!snap) return;
  try {
    const path = await save({
      defaultPath: `${snap.teams.home.name} - ${snap.teams.visiting.name}.dvw`,
      filters: [{ name: "DataVolley scout", extensions: ["dvw"] }],
    });
    if (!path) return;
    // The video's location is personal data: only in the file when asked.
    const withVideo =
      !!snap.videoPath &&
      (await ask(`Put the video's location in the file?\n\n${snap.videoPath}`, {
        title: "Export .dvw",
        kind: "info",
        okLabel: "Include it",
        cancelLabel: "Leave it out",
      }));
    await backend.exportDvw(path, withVideo);
    ui.result = { text: `Exported ${path}.`, bad: false };
  } catch (e) {
    ui.result = { text: String(e), bad: true };
  }
}

function rotate(side: Side) {
  const snap = ui.snapshot;
  const code = snap && rotationCode(side, snap.setterZone[side], true);
  if (code) void ui.commit(code);
}

/**
 * Picks the play `by` touches from the one picked (from the last, when
 * none is), shows it in the log and the video: stepping through plays to
 * mark where each went.
 */
function stepCode(by: 1 | -1): boolean {
  const snap = ui.snapshot;
  if (!snap) return false;
  if (ui.finding && ui.found.length) return stepFound(by);
  const touches = snap.groups.flatMap((g) => g.rows).filter((r) => r.kind === "touch");
  if (!touches.length) return false;
  const at = touches.findIndex((r) => r.id === ui.selected);
  const from = at < 0 ? touches.length : at;
  const r = touches[Math.max(0, Math.min(touches.length - 1, from + by))];
  if (!r) return false;
  ui.selected = r.id;
  const set = snap.groups.find((g) => g.rows.some((x) => x.id === r.id))?.set ?? snap.set;
  ui.shownSet = set === snap.set ? null : set;
  // After the log has drawn the set (and scrolled, if it changed).
  setTimeout(() => document.getElementById(`ev-${r.id}`)?.scrollIntoView({ block: "nearest" }));
  if (r.videoMs !== null && video.open) void video.seekTo(r.videoMs / 1000);
  return true;
}

/** Picks the search's next or previous match (#25) and shows it in the video. */
export function stepFound(by: 1 | -1): boolean {
  const snap = ui.snapshot;
  if (!snap) return false;
  const order = snap.groups.flatMap((g) => g.rows).map((r) => r.id);
  const id = stepMatch(ui.found, order, ui.selected, by);
  if (id === null) return false;
  pickCode(snap, id);
  return true;
}

/** Runs a keymap action; false if it doesn't apply right now. */
export function run(action: string): boolean {
  const snap = ui.snapshot;
  if (!snap) return false;
  switch (action) {
    case "undo":
      void ui.undo();
      return true;
    case "redo":
      void ui.redo();
      return true;
    case "focusCode":
      ui.focusCode += 1;
      return true;
    case "pointLeft":
      void ui.point(ui.order[0] ?? "home");
      return true;
    case "pointRight":
      void ui.point(ui.order[1] ?? "visiting");
      return true;
    case "timeoutHome":
      void ui.commit(timeoutCode("home"));
      return true;
    case "timeoutAway":
      void ui.commit(timeoutCode("visiting"));
      return true;
    case "subHome":
      ui.subFor = "home";
      return true;
    case "subAway":
      ui.subFor = "visiting";
      return true;
    case "rotateHome":
      rotate("home");
      return true;
    case "rotateAway":
      rotate("visiting");
      return true;
    case "swapSides":
      ui.swapped = !ui.swapped;
      return true;
    case "endView":
      ui.endView = !ui.endView;
      return true;
    case "layout":
      ui.setLayout(ui.layout === "watch" ? "coding" : "watch");
      return true;
    case "exportDvw":
      void exportDvw();
      return true;
    case "codePrev":
      return stepCode(-1);
    case "codeNext":
      return stepCode(1);
    case "openChecks":
      ui.openChecks();
      return true;
    case "find":
      ui.openFind();
      return true;
    case "modifyPicked":
    case "insertBeforePicked":
    case "insertAfterPicked":
    case "removePicked":
      return runPicked(action);
    default:
      return runVideo(action);
  }
}

/** Actions on the code picked in the log; false with none picked. */
function runPicked(action: string): boolean {
  const snap = ui.snapshot;
  const found = snap?.groups
    .map((g) => ({ g, r: g.rows.find((r) => r.id === ui.selected) }))
    .find((x) => x.r);
  const r = found?.r;
  if (!found || !r) return false;
  switch (action) {
    case "modifyPicked":
      if (r.kind === "goldenSet") return false;
      if (r.kind === "startingSix") {
        if (r.team === null) return false;
        ui.lineupScreen = found.g.set;
        ui.selected = null;
      } else ui.startEdit("correct", r.id, r.text);
      return true;
    case "insertBeforePicked":
      ui.startEdit("insert", r.id, r.text);
      return true;
    case "insertAfterPicked":
      ui.startEdit("after", r.id, r.text);
      return true;
    default:
      void ui.remove(r.id, r.text);
      return true;
  }
}

/** Video transport keys; false with no video open, so the key does its usual thing. */
function runVideo(action: string): boolean {
  if (!video.open) return false;
  switch (action) {
    case "videoPlay":
      void video.togglePlay();
      return true;
    case "videoBack":
      void video.seek(-JUMP);
      return true;
    case "videoForward":
      void video.seek(JUMP);
      return true;
    case "videoBackLong":
      void video.seek(-LONG_JUMP);
      return true;
    case "videoForwardLong":
      void video.seek(LONG_JUMP);
      return true;
    case "videoFrameBack":
      void video.step(true);
      return true;
    case "videoFrameForward":
      void video.step(false);
      return true;
    case "videoSlower":
      void video.speed(-1);
      return true;
    case "videoFaster":
      void video.speed(1);
      return true;
    case "clipPrev":
      video.stepClip(-1);
      return true;
    case "clipNext":
      video.stepClip(1);
      return true;
    default:
      return false;
  }
}
