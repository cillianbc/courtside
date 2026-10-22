// Clip windows for the clip playlist (PRD F-23, #25).
import type { Clip, Group } from "./snapshot";

/** A rally's span of video: its first and last code's times. */
export interface RallySpan {
  key: string;
  from: number;
  to: number;
}

/** Each code's rally span, by event id (rallies with no video time left out). */
export function rallySpans(groups: Group[]): Record<string, RallySpan> {
  const m: Record<string, RallySpan> = {};
  for (const g of groups) {
    const times = g.rows.filter((r) => r.videoMs !== null).map((r) => r.videoMs ?? 0);
    if (!times.length) continue;
    const span = { key: `${g.set}-${g.rally}`, from: Math.min(...times), to: Math.max(...times) };
    for (const r of g.rows) m[r.id] = span;
  }
  return m;
}

/**
 * `c` widened to its whole rally (the code times don't always line up with
 * the touch): from the rally's first code to its last, with the clip's own
 * lead-in and run-out.
 */
export function asRally(c: Clip, spans: Record<string, RallySpan>): Clip {
  const s = spans[c.id];
  if (!s) return c;
  return {
    ...c,
    startMs: Math.max(0, s.from - (c.videoMs - c.startMs)),
    endMs: s.to + (c.endMs - c.videoMs),
  };
}

/**
 * Every clip as its whole rally, each rally once: a later clip in a rally
 * already played gets an empty window, so the playlist moves straight past
 * it while the list's rows still line up with the clips.
 */
export function wholeRallies(clips: Clip[], spans: Record<string, RallySpan>): Clip[] {
  let last = "";
  let lastEnd = 0;
  return clips.map((c) => {
    const s = spans[c.id];
    if (!s) return c;
    if (s.key === last) return { ...c, startMs: lastEnd, endMs: lastEnd };
    const r = asRally(c, spans);
    last = s.key;
    lastEnd = r.endMs;
    return r;
  });
}
