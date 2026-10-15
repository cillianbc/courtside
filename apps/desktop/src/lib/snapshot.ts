// Mirrors crates/courtside-console/src/snapshot.rs (camelCase JSON).

export type Side = "home" | "visiting";
export type Severity = "info" | "warning" | "error";

export interface Pair<T> {
  home: T;
  visiting: T;
}

export interface PlayerView {
  number: number;
  name: string;
  code: string;
  /** `.dvw` role: 1 libero, 2 outside, 3 opposite, 4 middle, 5 setter. */
  role: number | null;
  libero: boolean;
  captain: boolean;
}

export interface TeamView {
  name: string;
  players: PlayerView[];
}

export interface SetView {
  number: number;
  score: Pair<number>;
  finished: boolean;
}

export type RowKind =
  | "touch"
  | "point"
  | "lineup"
  | "setterChange"
  | "setterPosition"
  | "substitution"
  | "timeout"
  | "setEnd"
  | "comment"
  | "removed"
  | "unrecognised"
  | "startingSix";

export interface Row {
  id: string;
  index: number;
  text: string;
  team: Side | null;
  kind: RowKind;
  /** The read-back, as in the command bar. */
  meaning: string;
  /** Written by the console (a point code or rotation), not typed. */
  auto: boolean;
  /** A correction of an earlier code, or a code inserted later. */
  edited: boolean;
  severity: Severity | null;
}

export interface Group {
  set: number;
  rally: number | null;
  serving: Side | null;
  wonBy: Side | null;
  scoreAfter: Pair<number> | null;
  closed: boolean;
  rows: Row[];
}

export interface IssueView {
  severity: Severity;
  rule: string;
  message: string;
  eventId: string;
  index: number;
  knockOn: boolean;
}

export interface Snapshot {
  matchId: string;
  teams: Pair<TeamView>;
  set: number;
  score: Pair<number>;
  serving: Side | null;
  sets: SetView[];
  court: Pair<(number | null)[]>;
  setter: Pair<number | null>;
  setterZone: Pair<number | null>;
  /** The team has no six on court: enter its lineup before play. */
  needsLineup: Pair<boolean>;
  timeouts: Pair<number>;
  substitutions: Pair<number>;
  groups: Group[];
  issues: IssueView[];
  events: number;
}

export interface MatchItem {
  id: string;
  home: string;
  visiting: string;
  date: string | null;
  sourceName: string | null;
  events: number;
}

export interface Readback {
  /** What the scout typed (a dot code gives two read-backs for one). */
  typed: string;
  /** The full code it becomes. */
  text: string;
  /** What was filled in: "skill predicted", "type from the serve"… */
  filled: string[];
  meaning: string;
  error: string | null;
}

export interface Preview {
  codes: Readback[];
  /** Every code can be read, so Enter will add them. */
  ok: boolean;
}

export const SIDES: readonly Side[] = ["home", "visiting"];

export function other(side: Side): Side {
  return side === "home" ? "visiting" : "home";
}

/** The team prefix of a code: `*` home, `a` visiting. */
export function prefix(side: Side): string {
  return side === "home" ? "*" : "a";
}
