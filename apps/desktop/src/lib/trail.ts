// The last console actions and backend commands, by name only (never codes
// or names): what a problem or crash report says came before it.

const KEEP = 40;
const trail: string[] = [];

/** Notes an action (`key:undo`, `commit`) for the next report. */
export function noteAction(name: string) {
  trail.push(name);
  if (trail.length > KEEP) trail.shift();
}

/** The last actions, oldest first. */
export function recentActions(): string[] {
  return [...trail];
}
