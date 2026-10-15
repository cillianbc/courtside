// What a change did to the sets, for the command bar.

import type { Side, Snapshot } from "./snapshot";

/**
 * What a change did to the sets, said after it so a set or match ended by
 * mistake is caught at once: a set won and waiting to be ended, a set
 * ended, the match over.
 */
export function setNews(before: Snapshot | null, after: Snapshot): string {
  const name = (side: Side) => after.teams[side].name;
  if (after.setEndDue && !before?.setEndDue) {
    const s = after.score;
    return ` Set ${after.set} is won at ${s.home}:${s.visiting}: Enter on an empty line or End set ends it; Undo if wrong.`;
  }
  const finished = (s: Snapshot | null) => s?.sets.filter((v) => v.finished).length ?? 0;
  const ended = finished(after) > finished(before) || (after.matchOver && !before?.matchOver);
  if (!ended) return "";
  const last = [...after.sets].reverse().find((v) => v.finished || after.matchOver);
  if (!last) return "";
  const winner: Side = last.score.home > last.score.visiting ? "home" : "visiting";
  const label = last.number === after.goldenSet ? "Golden set" : `Set ${last.number}`;
  let text = ` ${label} to ${name(winner)} ${last.score.home}:${last.score.visiting}.`;
  if (after.matchOver && !before?.matchOver) {
    const won = (side: Side) =>
      after.sets.filter(
        (v) =>
          v.number !== after.goldenSet &&
          (side === "home" ? v.score.home > v.score.visiting : v.score.visiting > v.score.home),
      ).length;
    text += ` Match over, ${name("home")} ${won("home")}–${won("visiting")} ${name("visiting")}.`;
  }
  return `${text} Undo if wrong.`;
}
