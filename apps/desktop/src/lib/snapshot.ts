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
  libero: boolean;
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
  | "unrecognised";

export interface Row {
  id: string;
  index: number;
  text: string;
  team: Side | null;
  kind: RowKind;
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

export const SIDES: readonly Side[] = ["home", "visiting"];

export function other(side: Side): Side {
  return side === "home" ? "visiting" : "home";
}

/** The team prefix of a code: `*` home, `a` visiting. */
export function prefix(side: Side): string {
  return side === "home" ? "*" : "a";
}
