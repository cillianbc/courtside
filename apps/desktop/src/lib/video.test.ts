import { describe, expect, it } from "vitest";
import type { Clip } from "./snapshot";
import { clipCue, stampTokens, stepSpeed } from "./video.svelte";

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

describe("clip playlists", () => {
  const clip: Clip = {
    id: "x",
    set: 1,
    score: { home: 0, visiting: 0 },
    team: "home",
    text: "*01SM-",
    meaning: "",
    videoMs: 13_000,
    startMs: 10_000,
    endMs: 16_000,
  };
  it("plays a clip to its end, then moves on", () => {
    expect(clipCue(clip, 10_000)).toBe("play");
    expect(clipCue(clip, 15_999)).toBe("play");
    expect(clipCue(clip, 16_000)).toBe("next");
    // A poll a little late still moves on.
    expect(clipCue(clip, 16_400)).toBe("next");
  });
  it("lets go when the scout moves the video elsewhere", () => {
    expect(clipCue(clip, 60_000)).toBe("left");
    expect(clipCue(clip, 2_000)).toBe("left");
  });
});
