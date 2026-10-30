// The command bar's one line: what the typed codes mean, or what's wrong.

import type { Preview } from "./snapshot";

export interface Hint {
  text: string;
  bad: boolean;
}

export const IDLE =
  "Enter adds the code · Esc clears · shorthand like a10 or 7S.a14# is filled in · several codes per line";

export function hintFor(line: string, preview: Preview | null): Hint {
  if (!line.trim()) return { text: IDLE, bad: false };
  if (!preview) return { text: "", bad: false };
  const wrong = preview.codes.find((c) => c.error);
  if (wrong) {
    const where = preview.codes.length > 1 ? `${wrong.text}: ` : "";
    return { text: where + (wrong.error ?? ""), bad: true };
  }
  // Shorthand shows the full code it becomes before what it means.
  const read = (c: Preview["codes"][number]) =>
    c.typed === c.text ? c.meaning : `${c.text} ${c.meaning}`;
  return { text: preview.codes.map(read).join(" · "), bad: false };
}
