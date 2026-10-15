// Team and roster management. Mirrors crates/courtside-console/src/manage.rs.

export interface PlayerDetail {
  /** Empty for a player not saved yet. */
  id: string;
  number: number;
  /** Empty: the season makes one from the names when saving. */
  code: string;
  lastName: string;
  firstName: string;
  nickname: string;
  /** `.dvw` role: 1 libero, 2 outside, 3 opposite, 4 middle, 5 setter. */
  role: number | null;
  libero: boolean;
  captain: boolean;
  foreign: boolean;
  birthDate: string | null;
  heightCm: number | null;
}

export interface TeamDetail {
  id: string;
  name: string;
  code: string;
  coach: string;
  assistant: string;
  matches: number;
  players: PlayerDetail[];
}

export interface TeamForm {
  name: string;
  code: string;
  coach: string;
  assistant: string;
}

export interface PlayerListItem {
  player: PlayerDetail;
  teamId: string;
  teamName: string;
  matches: number;
}

export const ROLES: readonly { value: number; label: string; short: string }[] = [
  { value: 5, label: "Setter", short: "S" },
  { value: 2, label: "Outside", short: "OH" },
  { value: 3, label: "Opposite", short: "OP" },
  { value: 4, label: "Middle", short: "MB" },
  { value: 1, label: "Libero", short: "L" },
];

export function roleShort(role: number | null): string {
  return ROLES.find((r) => r.value === role)?.short ?? "";
}

export function blankPlayer(number: number): PlayerDetail {
  return {
    id: "",
    number,
    code: "",
    lastName: "",
    firstName: "",
    nickname: "",
    role: null,
    libero: false,
    captain: false,
    foreign: false,
    birthDate: null,
    heightCm: null,
  };
}

/** The next free shirt number after the highest on the roster. */
export function nextNumber(rows: PlayerDetail[]): number {
  const used = new Set(rows.map((r) => r.number));
  let n = rows.length ? Math.max(...rows.map((r) => r.number)) + 1 : 1;
  if (n > 99) n = 0;
  while (used.has(n) && n < 99) n += 1;
  return n;
}

const letters = (s: string) =>
  [...s]
    .filter((c) => /[\p{L}\p{N}]/u.test(c))
    .slice(0, 3)
    .join("")
    .toUpperCase();

/**
 * The code the season will give a player saved without one (before it
 * adds `-2`, `-3`… for codes already taken): `SMI-JOH`.
 */
export function suggestCode(last: string, first: string, number: number): string {
  const l = letters(last);
  const f = letters(first);
  if (l && f) return `${l}-${f}`;
  return l || f || `P${number}`;
}

/** What is wrong with each row, by index; rows not listed are fine. */
export function rosterProblems(rows: PlayerDetail[]): Map<number, string> {
  const problems = new Map<number, string>();
  const seen = new Map<number, number>();
  const codes = new Map<string, number>();
  rows.forEach((r, i) => {
    if (!Number.isInteger(r.number) || r.number < 0 || r.number > 99) {
      problems.set(i, "Shirt numbers go from 0 to 99.");
    } else if (seen.has(r.number)) {
      problems.set(i, `#${r.number} is already on the roster.`);
    } else if (!r.lastName.trim() && !r.nickname.trim()) {
      problems.set(i, "Enter a last name.");
    }
    seen.set(r.number, i);
    const code = r.code.trim();
    if (code && codes.has(code) && !problems.has(i)) {
      problems.set(i, `Code ${code} is already on the roster.`);
    }
    if (code) codes.set(code, i);
  });
  return problems;
}

/** Pairs each player of `from` with the `into` player they most likely are. */
export function guessPairs(from: PlayerDetail[], into: PlayerDetail[]): Map<string, string> {
  const pairs = new Map<string, string>();
  const name = (p: PlayerDetail) => `${p.lastName} ${p.firstName}`.trim().toLowerCase();
  for (const p of from) {
    const match =
      into.find((q) => q.code && q.code === p.code) ??
      into.find((q) => name(q) && name(q) === name(p)) ??
      into.find(
        (q) => q.number === p.number && q.lastName.toLowerCase() === p.lastName.toLowerCase(),
      );
    if (match) pairs.set(p.id, match.id);
  }
  return pairs;
}

export interface TeamImportView {
  teamId: string;
  teamName: string;
  created: boolean;
  playersAdded: number;
  playersKept: number;
}

/** The scout-facing summary of a team file import. */
export function describeImport(r: TeamImportView): string {
  const players = (n: number) => `${n} player${n === 1 ? "" : "s"}`;
  return r.created
    ? `Added ${r.teamName} with ${players(r.playersAdded)}.`
    : `${r.teamName} was already here: added ${players(r.playersAdded)}, kept ${players(r.playersKept)} as they were.`;
}
