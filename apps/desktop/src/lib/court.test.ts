import { describe, expect, it } from "vitest";
import { LAYOUT, columns } from "./court";

// Zone at (row, col) of a half court drawn with `cols` columns.
function at(net: keyof typeof LAYOUT, row: number, col: number): number | undefined {
  return LAYOUT[net][row * columns(net) + col];
}

describe("court layout", () => {
  it("puts every zone once on each layout", () => {
    for (const zones of Object.values(LAYOUT)) {
      expect([...zones].sort()).toEqual([1, 2, 3, 4, 5, 6]);
    }
  });

  it("puts the front row at the net", () => {
    // Net on the right: right-hand column is 4, 3, 2 from top to bottom.
    expect([at("right", 0, 1), at("right", 1, 1), at("right", 2, 1)]).toEqual([4, 3, 2]);
    // Net at the top: first row is 4, 3, 2 from left to right.
    expect([at("top", 0, 0), at("top", 0, 1), at("top", 0, 2)]).toEqual([4, 3, 2]);
  });

  it("mirrors the two halves across the net", () => {
    expect(at("left", 0, 1)).toBe(at("right", 2, 0));
    expect(at("bottom", 0, 0)).toBe(at("top", 1, 2));
  });
});
