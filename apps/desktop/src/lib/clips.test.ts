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

  it("starts a rally where the one before was last coded", () => {
    // Rally 2 runs from rally 1's last code (16 s), less the lead-in.
    const c = asRally(clip("d2", 43_000), spans);
    expect([c.startMs, c.endMs]).toEqual([13_000, 49_000]);
  });

  it("keeps two rallies typed in one line apart", () => {
    // Both coded at 27:55.8 with the video paused; the first rally's play
    // lies after the point before them (27:38.9).
    const paused = rallySpans([
      group(5, [row("p5", 1_658_967)]),
      group(6, [row("s6", 1_675_842), row("k6", 1_675_842)]),
      group(7, [row("s7", 1_675_842), row("e7", 1_677_598)]),
    ]);
    expect(paused["k6"]).toMatchObject({ from: 1_658_967, to: 1_675_842 });
    expect(paused["e7"]).toMatchObject({ from: 1_675_842, to: 1_677_598 });
  });

  it("plays each rally once", () => {
    const list = wholeRallies([clip("d1", 14_000), clip("d2", 43_000), clip("d3", 46_000)], spans);
    expect(list.map((c) => [c.startMs, c.endMs])).toEqual([
      [7_000, 19_000],
      [13_000, 49_000],
      [49_000, 49_000],
    ]);
  });

  it("leaves a clip with no rally times as it is", () => {
    const c = clip("x", 5_000);
    expect(asRally(c, spans)).toBe(c);
  });
});
