// View state of the console that the backend doesn't own: court view,
// open tabs and the command line's text.

import * as backend from "./backend";
import { pointCode } from "./codes";
import { setNews } from "./news";
import type { Hint } from "./hint";
import { effectiveKeymap, type KeyBinding, type KeyText } from "./settings";
import { stampTokens, video } from "./video.svelte";
import type { Side, Snapshot } from "./snapshot";

/** The command bar is editing an earlier code rather than adding one. */
export interface Editing {
  /**
   * `correct`: modify the code; `insert`: add codes before it; `after`: add
   * codes after it; `rally`: add a missed rally before the rally it starts.
   */
  mode: "correct" | "insert" | "after" | "rally";
  id: string;
  /** The code being modified, or the one the new code goes before or after. */
  text: string;
}

/** A per-computer on/off remembered in the browser's storage (false when it can't be read). */
function remembered(key: string): boolean {
  try {
    return localStorage.getItem(key) === "1";
  } catch {
    return false;
  }
}

/** Remembers an on/off for [`remembered`]; a storage that can't be written is ignored. */
export function remember(key: string, on: boolean) {
  try {
    localStorage.setItem(key, on ? "1" : "0");
  } catch {
    // Private window or blocked storage: the choice lasts this session.
  }
}

export type Theme = "light" | "dark" | "system";

/** The theme this computer last chose ("system" when none or unreadable). */
function storedTheme(): Theme {
  try {
    const t = localStorage.getItem("courtside.theme");
    return t === "light" || t === "dark" ? t : "system";
  } catch {
    return "system";
  }
}

/** Puts the theme and boost on <html>, where the colour tokens read them. */
function applyLook(theme: Theme, boost: boolean) {
  if (typeof document === "undefined") return;
  const root = document.documentElement;
  if (theme === "system") root.removeAttribute("data-theme");
  else root.setAttribute("data-theme", theme);
  root.toggleAttribute("data-boost", boost);
}

class Ui {
  snapshot = $state<Snapshot | null>(null);
  /** Away team drawn first (left / bottom). */
  swapped = $state(false);
  /** Court drawn from the end line instead of the side. */
  endView = $state(false);
  /**
   * Coding: video on top, codes and team controls under it, the direction
   * court and checks on the right. Watch: the video fills the window beside
   * one narrow column. Remembered per computer.
   */
  layout = $state<"coding" | "watch">(remembered("courtside.watchLayout") ? "watch" : "coding");
  /** Light, dark or the system's; boost raises contrast over either. Per computer. */
  theme = $state<Theme>(storedTheme());
  boost = $state(remembered("courtside.boost"));
  sideTab = $state<"checks" | "set">("checks");
  /** The clip playlist shows where the code stream does. */
  clipsOpen = $state(false);
  /** Checks show where the code stream does (Video layout, v6 T8). */
  checksOpen = $state(false);
  /** Set whose codes the stream shows; `null` follows the set in progress. */
  shownSet = $state<number | null>(null);
  code = $state("");
  /** The result of the last action, shown in the command bar until the scout types again. */
  result = $state<Hint | null>(null);
  focusCode = $state(0);
  /** The code picked in the stream, by event id. */
  selected = $state<string | null>(null);
  editing = $state<Editing | null>(null);
  /** The team whose substitution picker is open. */
  subFor = $state<Side | null>(null);
  /**
   * The set whose starting lineups are open on the lineup screen, to modify
   * them. The screen also opens by itself while a team needs its six.
   */
  lineupScreen = $state<number | null>(null);
  /**
   * Menus and dialogs open over the window: the video's native view is
   * drawn above the page, so it hides while this is above 0.
   */
  covering = $state(0);
  /** This computer's keys, defaults filled in. */
  keymap = $state<KeyBinding[]>(effectiveKeymap([]));
  keyText = $state<KeyText[]>([]);

  get order(): Side[] {
    return this.swapped ? ["visiting", "home"] : ["home", "visiting"];
  }

  /**
   * The next snapshot comes from editing an earlier code: the code log
   * stays where it is instead of going to the latest rally. The log
   * clears it.
   */
  keepPlace = false;

  /**
   * Runs a change in the backend and shows the new state, or the error in
   * the command bar. Returns whether it worked. `inPlace`: an edit of an
   * earlier code, so the log keeps the set and place shown.
   */
  async apply(what: string, change: () => Promise<Snapshot>, inPlace = false): Promise<boolean> {
    // Any change cancels a point waiting for its second click.
    this.pointPending = null;
    try {
      const before = this.snapshot;
      const next = await change();
      if (inPlace) this.keepPlace = true;
      else this.shownSet = null;
      this.snapshot = next;
      this.result = { text: `${what}.${setNews(before, this.snapshot)}`, bad: false };
      return true;
    } catch (e) {
      this.result = { text: String(e), bad: true };
      return false;
    } finally {
      this.focusCode += 1;
    }
  }

  /** Ends a set whose score wins it (when set ends wait for the scout). */
  endSet(): Promise<boolean> {
    return this.apply("Set ended", backend.endSet);
  }

  /** Video times at the first keystroke of each code on the command line. */
  private stamps: (number | null)[] = [];
  /** When the command line's first key was typed. */
  private typedAt: Date | undefined;

  /** The command line changed: stamps codes typed anew. */
  keyed() {
    if (!this.code.trim()) return this.clearStamps();
    this.typedAt ??= new Date();
    this.stamps = stampTokens(this.code, this.stamps, video.nowMs());
  }

  clearStamps() {
    this.stamps = [];
    this.typedAt = undefined;
  }

  /**
   * Adds codes: `typed` from the command line, with the times its keys
   * were typed at; otherwise (buttons) stamped now.
   */
  commit(text: string, typed = false): Promise<boolean> {
    // A code typed while Checks stand in for the log brings the log back.
    if (typed) this.checksOpen = false;
    const stamps = stampTokens(text, typed ? this.stamps : [], video.nowMs());
    const at = typed ? this.typedAt : undefined;
    return this.apply(`Added ${text}`, () => backend.commit(text, stamps, at));
  }

  undo(): Promise<boolean> {
    return this.apply("Undone", backend.undo);
  }

  redo(): Promise<boolean> {
    return this.apply("Redone", backend.redo);
  }

  /** Starts modifying a code, or adding one before or after it, on the command bar. */
  startEdit(mode: Editing["mode"], id: string, text: string) {
    this.editing = { mode, id, text };
    this.code = mode === "correct" ? text : "";
    this.clearStamps();
    this.result = null;
    this.focusCode += 1;
  }

  cancelEdit() {
    this.editing = null;
    this.code = "";
    this.clearStamps();
  }

  /** Where codes on the command bar go in: the code they go before, if not at the end. */
  insertingBefore(): string | null {
    const e = this.editing;
    if (!e || e.mode === "correct") return null;
    return e.mode === "after" ? this.codeAfter(e.id) : e.id;
  }

  /** Enter on the command bar: add, correct or insert. */
  async submit(text: string): Promise<boolean> {
    const e = this.editing;
    let ok: boolean;
    if (!e) ok = await this.commit(text, true);
    else if (e.mode === "correct")
      ok = await this.apply(
        `Modified ${e.text} to ${text}`,
        () => backend.correct(e.id, text),
        true,
      );
    else if (e.mode === "rally")
      ok = await this.apply(
        `Added a rally before ${e.text}`,
        () => backend.insertBefore(e.id, text),
        true,
      );
    else if (e.mode === "insert")
      ok = await this.apply(
        `Added ${text} before ${e.text}`,
        () => backend.insertBefore(e.id, text),
        true,
      );
    else {
      // After the last code there is nothing to go before: add it as typed.
      const next = this.codeAfter(e.id);
      ok = next
        ? await this.apply(
            `Added ${text} after ${e.text}`,
            () => backend.insertBefore(next, text),
            true,
          )
        : await this.commit(text, true);
    }
    if (ok) {
      this.clearStamps();
      this.editing = null;
      this.selected = null;
    }
    return ok;
  }

  /** A point with no rally codes waiting for a second click, by team. */
  pointPending = $state<Side | null>(null);

  /**
   * `side` wins the rally in progress; the console rotates and ends the set
   * as usual. With nothing coded in the rally the last one is already over
   * (a deciding code wrote its point), so the first click only says so and
   * a second click on the same team gives a point with no rally (a penalty).
   */
  point(side: Side): Promise<boolean> {
    const snap = this.snapshot;
    // A missed rally going in: the point ends it (its score is written
    // where the rally lands).
    if (snap && this.editing?.mode === "rally") {
      return this.submit(`${this.code.trim()} ${pointCode(side, { home: 0, visiting: 0 })}`);
    }
    if (!snap || snap.matchOver || snap.needsLineup.home || snap.needsLineup.visiting)
      return Promise.resolve(false);
    const open = snap.groups.at(-1);
    const empty = !open || (!open.closed && open.rows.length === 0);
    if (empty && this.pointPending !== side) {
      this.pointPending = side;
      const last = [...snap.groups].reverse().find((g) => g.closed);
      const name = snap.teams[side].name;
      const decided = [...(last?.rows ?? [])].reverse().find((r) => !r.auto && r.kind !== "point");
      const before = last?.wonBy
        ? `The last rally already went to ${snap.teams[last.wonBy].name}` +
          (decided ? ` (${decided.text})` : "") +
          (last.wonBy === side ? "." : `: undo or modify it if ${name} won.`)
        : "No codes in this rally yet.";
      this.result = {
        text: `${before} Click again to give ${name} a point with no rally (e.g. a penalty).`,
        bad: true,
      };
      return Promise.resolve(false);
    }
    this.pointPending = null;
    return this.commit(pointCode(side, snap.score));
  }

  /** The id of the code after `id` in the whole match, if any. */
  codeAfter(id: string): string | null {
    const rows = (this.snapshot?.groups ?? []).flatMap((g) => g.rows);
    const i = rows.findIndex((r) => r.id === id);
    return i >= 0 ? (rows[i + 1]?.id ?? null) : null;
  }

  remove(id: string, text: string): Promise<boolean> {
    this.selected = null;
    return this.apply(`Removed ${text}`, () => backend.remove(id), true);
  }

  /** Switches the console's layout and remembers it on this computer. */
  setLayout(layout: "coding" | "watch") {
    this.layout = layout;
    remember("courtside.watchLayout", layout === "watch");
  }

  /**
   * Shows the checks without moving the video: in place of the code log in
   * the Video layout, as the side panel's tab in the Coding layout.
   */
  openChecks() {
    this.sideTab = "checks";
    if (this.layout === "watch") {
      this.clipsOpen = false;
      this.checksOpen = true;
    }
  }

  setTheme(theme: Theme) {
    this.theme = theme;
    try {
      localStorage.setItem("courtside.theme", theme);
    } catch {
      // Blocked storage: the choice lasts this session.
    }
    applyLook(this.theme, this.boost);
  }

  setBoost(on: boolean) {
    this.boost = on;
    remember("courtside.boost", on);
    applyLook(this.theme, this.boost);
  }

  /** Loads this computer's keys (at start, after opening a season or saving them). */
  async loadKeymap() {
    try {
      const s = await backend.keys();
      this.keymap = effectiveKeymap(s?.keymap ?? []);
      this.keyText = s?.keyText ?? [];
    } catch {
      this.keymap = effectiveKeymap([]);
      this.keyText = [];
    }
  }

  /** Puts `text` in the command bar and focuses it. */
  startCode(text: string) {
    this.code = text;
    this.clearStamps();
    this.keyed();
    this.focusCode += 1;
  }
}

export const ui = new Ui();
applyLook(ui.theme, ui.boost);
