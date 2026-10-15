// View state of the console that the backend doesn't own: court view,
// open tabs and the command line's text.

import * as backend from "./backend";
import type { Hint } from "./hint";
import type { Side, Snapshot } from "./snapshot";

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

  /** Puts `text` in the command bar and focuses it. */
  startCode(text: string) {
    this.code = text;
    this.focusCode += 1;
  }
}

export const ui = new Ui();
