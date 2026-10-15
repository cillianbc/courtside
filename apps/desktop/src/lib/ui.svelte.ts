// View state of the console that the backend doesn't own: court view,
// open tabs and the command line's text.

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
  message = $state("");
  focusCode = $state(0);

  get order(): Side[] {
    return this.swapped ? ["visiting", "home"] : ["home", "visiting"];
  }

  /** Puts `text` in the command bar and focuses it. */
  startCode(text: string) {
    this.code = text;
    this.focusCode += 1;
  }
}

export const ui = new Ui();
