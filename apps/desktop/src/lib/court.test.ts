import { describe, expect, it } from "vitest";
import { LAYOUT, cellAt, cellPosition, columns, fromDrawing, toDrawing } from "./court";

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

describe("play positions", () => {
  it("reads .dvw grid cells as openvolley does", () => {
    const p = cellPosition(4868);
    expect(p?.x).toBeCloseTo(2.65625, 6);
    expect(p?.y).toBeCloseTo(3.3518519, 6);
    expect(cellPosition(0)).toBeNull();
    expect(cellPosition(null)).toBeNull();
    for (const cell of [1, 100, 101, 4868, 10100]) expect(cellAt(cellPosition(cell)!)).toBe(cell);
  });

  it("draws the first team's zone 4 where the zone grid has it", () => {
    // Zone 4 at the net, on the team's left: x 1, y just below the net.
    const zone4 = { x: 1, y: 3.2 };
    // Side view: first team on the left, net on its right, zone 4 at the top.
    const side = toDrawing(zone4, false, false);
    expect(side.x).toBeCloseTo(3.2);
    expect(side.y).toBeCloseTo(1);
    // End view: first team at the top, net below it, zone 4 on the right.
    const end = toDrawing(zone4, false, true);
    expect(end.x).toBeCloseTo(3);
    expect(end.y).toBeCloseTo(3.2);
    // The second team's zone 4 sits across the net, at the bottom (side)
    // or on the left (end).
    expect(toDrawing(zone4, true, false).y).toBeCloseTo(3);
    expect(toDrawing(zone4, true, false).x).toBeGreaterThan(3.5);
    expect(toDrawing(zone4, true, true).x).toBeCloseTo(1);
  });

  it("undoes the drawing", () => {
    for (const mirrored of [false, true])
      for (const end of [false, true]) {
        const p = { x: 1.3, y: 5.7 };
        const back = fromDrawing(toDrawing(p, mirrored, end), mirrored, end);
        expect(back.x).toBeCloseTo(p.x);
        expect(back.y).toBeCloseTo(p.y);
      }
  });
});
