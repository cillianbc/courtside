import { describe, expect, it } from "vitest";
import { stepSpeed } from "./video.svelte";

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
