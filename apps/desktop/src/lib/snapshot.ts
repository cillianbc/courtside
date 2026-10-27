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
  /** The season team's id (for the team editor). */
  id: string;
  /** The team's colour preset ([`TEAM_COLOURS`] in teams.ts); empty for the default. */
  colour: string;
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
  | "startingSix"
  | "goldenSet";

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
  /** Where the code is in the match video, in ms. */
  videoMs: number | null;
  /** Where the play started and ended: `.dvw` grid cells in the play's frame (`court.ts`). */
  start: number | null;
  end: number | null;
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
  /** For a knock-on: the code whose issue this one follows from. */
  causeId: string | null;
}

export interface SetStart {
  set: number;
  home: StartingView | null;
  visiting: StartingView | null;
}

/** The six and setter `side` started `set` with, if known. */
export function startOf(snap: Snapshot, set: number, side: Side): StartingView | null {
  return snap.starting.find((s) => s.set === set)?.[side] ?? null;
}

export interface StartingView {
  /** Shirt numbers in zones 1–6. */
  court: (number | null)[];
  setter: number | null;
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
  /** A team has won the match (and the golden set, if played); no set follows. */
  matchOver: boolean;
  /** The set played as a golden set, if one was started. */
  goldenSet: number | null;
  /** The set's score wins it but it isn't ended yet (confirm or undo). */
  setEndDue: boolean;
  /** While a set end is due: its length in minutes from the clock, to confirm or change. */
  setEndMinutes: number | null;
  /** Substitutions allowed per team and set. */
  substitutionLimit: number;
  /** Each team's six and setter as the set in progress started. */
  /** Each set's starting six and setter per team, once anything is coded in it. */
  starting: SetStart[];
  timeouts: Pair<number>;
  substitutions: Pair<number>;
  groups: Group[];
  issues: IssueView[];
  events: number;
  /** The match video's path: from the imported file or opened with it. */
  videoPath: string | null;
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

/** Which touches a clip playlist holds; empty fields match all (`clips.rs`). */
export interface ClipFilter {
  team: Side | null;
  player: number | null;
  /** Skill letter. */
  skill: string | null;
  /** Evaluation symbols, any of which matches; "" for all. */
  evaluations: string;
  /** The touching team's setter zone, 1–6. */
  rotation: number | null;
  phase: "serve" | "reception" | "transition" | null;
  set: number | null;
  /** Text the code contains, as written (`D=`, `a12`); "" for all. */
  text: string;
}

export interface Clip {
  /** The code's event id, as in the stream's rows. */
  id: string;
  set: number;
  score: Pair<number>;
  team: Side;
  text: string;
  meaning: string;
  videoMs: number;
  startMs: number;
  endMs: number;
}

export interface Clips {
  clips: Clip[];
  /** Touches the filter picks that have no video time. */
  withoutVideo: number;
}
