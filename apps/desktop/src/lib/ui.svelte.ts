// View state of the console that the backend doesn't own: court view,
// open tabs and the command line's text.

import * as backend from "./backend";
import type { Hint } from "./hint";
import { effectiveKeymap, type KeyBinding } from "./settings";
import type { Side, Snapshot } from "./snapshot";

/** The command bar is editing an earlier code rather than adding one. */
export interface Editing {
  /** `correct`: modify the code; `insert`: add one before it; `after`: add one after it. */
  mode: "correct" | "insert" | "after";
  id: string;
  /** The code being modified, or the one the new code goes before or after. */
  text: string;
}

class Ui {
  snapshot = $state<Snapshot | null>(null);
  /** Away team drawn first (left / bottom). */
  swapped = $state(false);
  /** Court drawn from the end line instead of the side. */
  endView = $state(false);
  sideTab = $state<"checks" | "set">("checks");
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
  /** The team whose lineup is being modified. */
  lineupFor = $state<Side | null>(null);
  /** The season's keys, defaults filled in. */
  keymap = $state<KeyBinding[]>(effectiveKeymap([]));

  get order(): Side[] {
    return this.swapped ? ["visiting", "home"] : ["home", "visiting"];
  }

  /**
   * Runs a change in the backend and shows the new state, or the error in
   * the command bar. Returns whether it worked.
   */
  async apply(what: string, change: () => Promise<Snapshot>): Promise<boolean> {
    try {
      this.snapshot = await change();
      this.shownSet = null;
      this.result = { text: `${what}.`, bad: false };
      return true;
    } catch (e) {
      this.result = { text: String(e), bad: true };
      return false;
    } finally {
      this.focusCode += 1;
    }
  }

  /** Adds codes as if typed on the command bar. */
  commit(text: string): Promise<boolean> {
    return this.apply(`Added ${text}`, () => backend.commit(text));
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
    this.result = null;
    this.focusCode += 1;
  }

  cancelEdit() {
    this.editing = null;
    this.code = "";
  }

  /** Enter on the command bar: add, correct or insert. */
  async submit(text: string): Promise<boolean> {
    const e = this.editing;
    let ok: boolean;
    if (!e) ok = await this.commit(text);
    else if (e.mode === "correct")
      ok = await this.apply(`Modified ${e.text} to ${text}`, () => backend.correct(e.id, text));
    else if (e.mode === "insert")
      ok = await this.apply(`Added ${text} before ${e.text}`, () =>
        backend.insertBefore(e.id, text),
      );
    else {
      // After the last code there is nothing to go before: add it as typed.
      const next = this.codeAfter(e.id);
      ok = next
        ? await this.apply(`Added ${text} after ${e.text}`, () => backend.insertBefore(next, text))
        : await this.commit(text);
    }
    if (ok) {
      this.editing = null;
      this.selected = null;
    }
    return ok;
  }

  /** The id of the code after `id` in the whole match, if any. */
  codeAfter(id: string): string | null {
    const rows = (this.snapshot?.groups ?? []).flatMap((g) => g.rows);
    const i = rows.findIndex((r) => r.id === id);
    return i >= 0 ? (rows[i + 1]?.id ?? null) : null;
  }

  remove(id: string, text: string): Promise<boolean> {
    this.selected = null;
    return this.apply(`Removed ${text}`, () => backend.remove(id));
  }

  /** Loads the season's keymap (after opening a season or saving settings). */
  async loadKeymap() {
    try {
      const s = await backend.settings();
      this.keymap = effectiveKeymap(s?.keymap ?? []);
    } catch {
      this.keymap = effectiveKeymap([]);
    }
  }

  /** Puts `text` in the command bar and focuses it. */
  startCode(text: string) {
    this.code = text;
    this.focusCode += 1;
  }
}

export const ui = new Ui();
