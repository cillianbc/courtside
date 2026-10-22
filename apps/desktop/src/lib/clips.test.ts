import { describe, expect, it } from "vitest";
import { asRally, rallySpans, wholeRallies } from "./clips";
import type { Clip, Group, Row } from "./snapshot";

const row = (id: string, videoMs: number | null): Row => ({ id, videoMs }) as unknown as Row;
const group = (rally: number, rows: Row[]): Group => ({ set: 1, rally, rows }) as unknown as Group;
const clip = (id: string, videoMs: number): Clip =>
  ({ id, videoMs, startMs: videoMs - 3000, endMs: videoMs + 3000 }) as unknown as Clip;

describe("clip windows", () => {
  const spans = rallySpans([
    group(1, [row("s1", 10_000), row("d1", 14_000), row("p1", null), row("a1", 16_000)]),
    group(2, [row("s2", 40_000), row("d2", 43_000), row("d3", 46_000)]),
  ]);

  it("widens a clip to its rally with its own lead-in and run-out", () => {
    const c = asRally(clip("d1", 14_000), spans);
    expect([c.startMs, c.endMs]).toEqual([7_000, 19_000]);
  });

  it("plays each rally once", () => {
    const list = wholeRallies([clip("d1", 14_000), clip("d2", 43_000), clip("d3", 46_000)], spans);
    expect(list.map((c) => [c.startMs, c.endMs])).toEqual([
      [7_000, 19_000],
      [37_000, 49_000],
      [49_000, 49_000],
    ]);
  });

  it("leaves a clip with no rally times as it is", () => {
    const c = clip("x", 5_000);
    expect(asRally(c, spans)).toBe(c);
  });
});
