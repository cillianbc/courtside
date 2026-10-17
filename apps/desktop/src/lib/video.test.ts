import { describe, expect, it } from "vitest";
import { stampTokens, stepSpeed } from "./video.svelte";

describe("video speed", () => {
  it("steps through the speeds without wrapping", () => {
    expect(stepSpeed(1, 1)).toBe(1.5);
    expect(stepSpeed(1, -1)).toBe(0.5);
    expect(stepSpeed(2, 1)).toBe(2);
    expect(stepSpeed(0.25, -1)).toBe(0.25);
    // A speed set elsewhere steps from normal.
    expect(stepSpeed(0.75, 1)).toBe(1.5);
  });
});

describe("keystroke stamps", () => {
  it("stamps each code when its first key is typed", () => {
    let t = stampTokens("a", [], 1000);
    expect(t).toEqual([1000]);
    t = stampTokens("a16RM+ ", t, 1400);
    expect(t).toEqual([1000]);
    t = stampTokens("a16RM+ a1", t, 1800);
    expect(t).toEqual([1000, 1800]);
    // Rubbing out a code drops its stamp; typing it again stamps anew.
    t = stampTokens("a16RM+", t, 2000);
    expect(t).toEqual([1000]);
    expect(stampTokens("a16RM+ a14", t, null)).toEqual([1000, null]);
    expect(stampTokens("  ", t, 3000)).toEqual([]);
  });
});
