// The command bar's one line: what the typed codes mean, or what's wrong.

import type { Preview } from "./snapshot";

export interface Hint {
  text: string;
  bad: boolean;
}

export const IDLE = "Enter adds the code · Esc clears · several codes can go on one line";

export function hintFor(line: string, preview: Preview | null): Hint {
  if (!line.trim()) return { text: IDLE, bad: false };
  if (!preview) return { text: "", bad: false };
  const wrong = preview.codes.find((c) => c.error);
  if (wrong) {
    const where = preview.codes.length > 1 ? `${wrong.text}: ` : "";
    return { text: where + (wrong.error ?? ""), bad: true };
  }
  return { text: preview.codes.map((c) => c.meaning).join(" · "), bad: false };
}
