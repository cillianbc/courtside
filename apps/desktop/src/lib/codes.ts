// Codes the console's buttons and forms write. They go through the same
// commit as typed codes.

import { prefix, type Side } from "./snapshot";

const two = (n: number) => String(n).padStart(2, "0");

export function timeoutCode(side: Side): string {
  return `${prefix(side)}T`;
}

export function substitutionCode(side: Side, off: number, on: number): string {
  return `${prefix(side)}c${two(off)}:${two(on)}`;
}

/**
 * Moves the setter one zone on (forward: 2 to 1, 1 to 6) or back. `null`
 * while the setter's zone isn't known.
 */
export function rotationCode(
  side: Side,
  setterZone: number | null,
  forward: boolean,
): string | null {
  if (setterZone === null) return null;
  const z = setterZone;
  const next = forward ? (z === 1 ? 6 : z - 1) : z === 6 ? 1 : z + 1;
  return `${prefix(side)}z${next}`;
}

export type SixResult = { court: number[] } | { error: string };

/** Reads six shirt numbers for zones 1–6, separated by spaces or commas. */
export function parseSix(text: string): SixResult {
  const parts = text.split(/[\s,]+/).filter(Boolean);
  if (parts.length !== 6) return { error: `Six numbers for zones 1–6, got ${parts.length}.` };
  const court = parts.map(Number);
  const bad = parts.find((p, i) => !/^\d{1,2}$/.test(p) || Number.isNaN(court[i]));
  if (bad !== undefined) return { error: `${bad} isn't a shirt number.` };
  return { court };
}
