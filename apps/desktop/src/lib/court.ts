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
