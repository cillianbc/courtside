// Court geometry: which zone goes in which cell, by where the net is.

/** Where a half court's net is, as drawn. */
export type Net = "right" | "left" | "top" | "bottom";

/**
 * Zones 1–6 in reading order (row by row) for a half court. A team facing
 * the net has zone 4 front left, 2 front right and 1 back right.
 */
export const LAYOUT: Record<Net, readonly number[]> = {
  right: [5, 4, 6, 3, 1, 2],
  left: [2, 1, 3, 6, 4, 5],
  top: [4, 3, 2, 5, 6, 1],
  bottom: [1, 6, 5, 2, 3, 4],
};

/** Columns in the grid for a half court with its net on this side. */
export function columns(net: Net): number {
  return net === "top" || net === "bottom" ? 3 : 2;
}

/**
 * A position in court units (as `vb_engine::court`): the court is x 0.5–3.5
 * by y 0.5–6.5 with the net at y 3.5. In a play's own frame its team's
 * half is at the bottom (y < 3.5) and its zone 4 on the left.
 */
export interface Pt {
  x: number;
  y: number;
}

/** The centre of a `.dvw` grid cell (columns 4–6 of a `[3SCOUT]` line). */
export function cellPosition(cell: number | null): Pt | null {
  if (cell === null || cell < 1 || cell > 10100) return null;
  const xi = ((cell - 1) % 100) + 1;
  const yi = Math.floor((cell - 1) / 100) + 1;
  return { x: 0.5 + ((xi - 11) / 80) * 3 + 3 / 160, y: 0.5 + ((yi - 11) / 81) * 6 + 3 / 81 };
}

/** The grid cell a position falls in ([`cellPosition`] undone). */
export function cellAt(p: Pt): number {
  const xi = Math.min(100, Math.max(1, Math.floor(((p.x - 0.5) * 80) / 3) + 11));
  const yi = Math.min(101, Math.max(1, Math.floor(((p.y - 0.5) * 81) / 6) + 11));
  return xi + (yi - 1) * 100;
}

/**
 * Where a point of a play lands in the drawing. `mirrored`: the play's
 * team is the second one drawn (right in the side view, bottom in the end
 * view). Side view: the first team's baseline on the left, its zone 4 at
 * the top. End view: its baseline at the top, its zone 4 on the right.
 */
export function toDrawing(p: Pt, mirrored: boolean, end: boolean): Pt {
  const q = mirrored ? { x: 4 - p.x, y: 7 - p.y } : p;
  return end ? { x: 4 - q.x, y: q.y } : { x: q.y, y: q.x };
}

/** The play's own position for a point of the drawing ([`toDrawing`] undone). */
export function fromDrawing(s: Pt, mirrored: boolean, end: boolean): Pt {
  const q = end ? { x: 4 - s.x, y: s.y } : { x: s.y, y: s.x };
  return mirrored ? { x: 4 - q.x, y: 7 - q.y } : q;
}
