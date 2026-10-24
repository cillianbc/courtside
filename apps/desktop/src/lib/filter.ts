// The filter Clips and the code log's search share (#25): which codes, by
// team, player, skill, evaluation, rotation, phase, set and code text.

import type { ClipFilter, Snapshot } from "./snapshot";

export const SKILLS: [string, string][] = [
  ["S", "Serve"],
  ["R", "Reception"],
  ["E", "Set"],
  ["A", "Attack"],
  ["B", "Block"],
  ["D", "Dig"],
  ["F", "Free ball"],
];

export function emptyFilter(): ClipFilter {
  return {
    team: null,
    player: null,
    skill: null,
    evaluations: "",
    rotation: null,
    phase: null,
    set: null,
    text: "",
  };
}

/** Whether anything besides the code text is set. */
export function hasFields(f: ClipFilter): boolean {
  return (
    f.team !== null ||
    f.player !== null ||
    f.skill !== null ||
    f.evaluations !== "" ||
    f.rotation !== null ||
    f.phase !== null ||
    f.set !== null
  );
}

/** The filter in words, most telling part first ("Digs", "both teams", …). */
export function filterSummary(f: ClipFilter, snap: Snapshot): string[] {
  const skill = SKILLS.find(([k]) => k === f.skill)?.[1];
  const parts = [skill ? `${skill}s` : "All codes"];
  parts.push(f.team ? snap.teams[f.team].name : "both teams");
  if (f.player !== null) parts.push(`#${f.player}`);
  if (f.evaluations) parts.push(f.evaluations.split("").join(" "));
  if (f.rotation !== null) parts.push(`setter in ${f.rotation}`);
  if (f.phase)
    parts.push({ serve: "serve", reception: "side-out", transition: "transition" }[f.phase]);
  parts.push(f.set === null ? "all sets" : `set ${f.set}`);
  if (f.text) parts.push(`“${f.text}”`);
  return parts;
}

/**
 * The match `by` steps (±1) from the picked code: the next or previous one
 * found, from where the picked code is in the log (`order`, every code's
 * id in match order) when it isn't one; round to the other end after the
 * last. `null` with nothing found.
 */
export function stepMatch(
  found: string[],
  order: string[],
  picked: string | null,
  by: 1 | -1,
): string | null {
  if (!found.length) return null;
  const at = picked === null ? -1 : found.indexOf(picked);
  if (at >= 0) return found[(at + by + found.length) % found.length] ?? null;
  const pos = new Map(order.map((id, i) => [id, i]));
  const from = picked === null ? (by > 0 ? -1 : order.length) : (pos.get(picked) ?? -1);
  const next =
    by > 0
      ? found.find((id) => (pos.get(id) ?? -1) > from)
      : [...found].reverse().find((id) => (pos.get(id) ?? -1) < from);
  return next ?? (by > 0 ? found[0] : found[found.length - 1]) ?? null;
}
