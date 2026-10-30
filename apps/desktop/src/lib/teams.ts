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
  weightKg: number | null;
  /** Attack reach. */
  reachCm: number | null;
}

export interface PersonView {
  lastName: string;
  firstName: string;
}

/** A team's staff (VolleyStation's roles; .dvw and .sq hold the first two). */
export interface StaffView {
  coach: PersonView;
  assistant: PersonView;
  assistant2: PersonView;
  physio: PersonView;
  doctor: PersonView;
}

export interface VenueView {
  hall: string;
  city: string;
}

export interface TeamDetail extends TeamForm {
  id: string;
  matches: number;
  players: PlayerDetail[];
}

export interface TeamForm {
  name: string;
  /** Empty: made from the name when saving. */
  code: string;
  /** Up to five characters (.sq and .vst). */
  shortCode: string;
  staff: StaffView;
  /** Home hall and city: a new match's venue when the team is at home. */
  hall: string;
  city: string;
  /** Kit colours, `#rrggbb` or empty; the shirt colour goes in the .dvw. */
  shirtColour: string;
  liberoColour: string;
  /** The screen colour: a preset's key ([`TEAM_COLOURS`]); empty for the default. */
  colour: string;
}

export const STAFF_ROLES: readonly { key: keyof StaffView; label: string; note?: string }[] = [
  { key: "coach", label: "Head coach", note: "In the .dvw and .sq" },
  { key: "assistant", label: "Assistant coach", note: "In the .dvw and .sq" },
  { key: "assistant2", label: "Second assistant" },
  { key: "physio", label: "Physio" },
  { key: "doctor", label: "Doctor" },
];

const noOne = (): PersonView => ({ lastName: "", firstName: "" });

/** The editor's starting form: a copy of `team`, or an empty team. */
export function teamForm(team: TeamDetail | null): TeamForm {
  return {
    name: team?.name ?? "",
    code: team?.code ?? "",
    shortCode: team?.shortCode ?? "",
    staff: team
      ? structuredClone(team.staff)
      : {
          coach: noOne(),
          assistant: noOne(),
          assistant2: noOne(),
          physio: noOne(),
          doctor: noOne(),
        },
    hall: team?.hall ?? "",
    city: team?.city ?? "",
    shirtColour: team?.shirtColour ?? "",
    liberoColour: team?.liberoColour ?? "",
    colour: team?.colour ?? "",
  };
}

/** "First Last", as people are named on screen. */
export function personName(p: PersonView): string {
  return [p.firstName.trim(), p.lastName.trim()].filter(Boolean).join(" ");
}

/**
 * Team colours a team may pick (v6 T13): each a light and a dark tone that
 * keep 4.5:1 on the panels, and far enough from the error, warning,
 * selection and setter colours. Contrast on the light / dark panel noted.
 */
export const TEAM_COLOURS: readonly { key: string; label: string; light: string; dark: string }[] =
  [
    { key: "blue", label: "Blue", light: "#1d5fbf", dark: "#6aa3ff" }, // 6.10 / 6.42
    { key: "navy", label: "Navy", light: "#2b3f8f", dark: "#8fa2ef" }, // 9.50 / 6.64
    { key: "green", label: "Green", light: "#2e7629", dark: "#6fcf6a" }, // 5.62 (4.68 on sunk) / 8.37
    { key: "orange", label: "Orange", light: "#b04a12", dark: "#f39a62" }, // 5.48 / 7.44
    { key: "red-orange", label: "Red-orange", light: "#b8361e", dark: "#ff8a70" }, // 5.86 / 7.06
    { key: "brown", label: "Brown", light: "#7a4a26", dark: "#d39a6e" }, // 7.41 / 6.67
    { key: "charcoal", label: "Charcoal", light: "#2b2f36", dark: "#c9d1db" }, // 13.4 / 10.6
  ];

/** The preset for `key`, if it is one. */
export function teamColour(key: string) {
  return TEAM_COLOURS.find((c) => c.key === key) ?? null;
}

/**
 * Puts a match's team colours on <html> (the colour tokens read them). The
 * visiting team keeps its default when both picked the same colour, so the
 * two teams never look alike.
 */
export function applyTeamColours(home: string, visiting: string) {
  if (typeof document === "undefined") return;
  const root = document.documentElement;
  const set = (side: "home" | "away", key: string) => {
    const c = teamColour(key);
    root.toggleAttribute(`data-${side}-colour`, !!c);
    if (c) {
      root.style.setProperty(`--${side}-l`, c.light);
      root.style.setProperty(`--${side}-d`, c.dark);
    }
  };
  set("home", home);
  set("away", visiting && visiting === home ? "" : visiting);
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
    weightKg: null,
    reachCm: null,
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
 * adds `-2`, `-3`… for codes already taken): `ARD-JON`.
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
