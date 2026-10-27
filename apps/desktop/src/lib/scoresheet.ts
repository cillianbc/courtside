// Scoresheets (issue #57). Mirrors crates/vb-scoresheet (the sheet) and
// crates/courtside-console/src/sheet.rs (how it meets the season and match).

import type { Pair, Side } from "./snapshot";

export interface SheetPlayer {
  number: number;
  lastName: string;
  firstName: string;
  /** The licence number in brackets after the name. */
  id: string;
  libero: boolean;
  captain: boolean;
}

export interface SheetStaff {
  /** `C` coach, `AC` assistant coach, … */
  role: string;
  lastName: string;
  firstName: string;
  id: string;
}

export interface SheetTeam {
  name: string;
  letter: string;
  players: SheetPlayer[];
  staff: SheetStaff[];
}

export interface Substitution {
  team: number;
  position: number;
  playerOut: number;
  playerIn: number;
  /** The substituting team's own score first. */
  scoreIn: [number, number] | null;
  scoreBack: [number, number] | null;
}

export interface SheetSet {
  number: number;
  start: string;
  end: string;
  servedFirst: number | null;
  /** By team index: zones 1–6 (I–VI). */
  lineups: [number[], number[]];
  substitutions: Substitution[];
  score: [number, number] | null;
  minutes: number | null;
}

export interface Scoresheet {
  source: string;
  competition: string;
  category: string;
  matchNumber: string;
  day: string;
  date: string | null;
  time: string;
  city: string;
  hall: string;
  country: string;
  /** Home first, as the sheet's title has them. */
  teams: [SheetTeam, SheetTeam];
  sets: SheetSet[];
  referees: string[];
  scorer: string;
  assistantScorer: string;
}

export interface TeamMatch {
  teamId: string | null;
  teamName: string | null;
  newPlayers: number[];
  differing: string[];
}

export interface SheetPreview {
  sheet: Scoresheet;
  teams: [TeamMatch, TeamMatch];
}

export interface SheetChoice {
  /** Per sheet team: the season team to use; null adds the sheet's team. */
  teamIds: [string | null, string | null];
}

export interface SheetSetView {
  set: number;
  sixes: Pair<number[]>;
  serving: Side | null;
  score: Pair<number> | null;
}

export interface SheetSummary {
  source: string;
  sets: SheetSetView[];
}

export interface SheetCheck {
  sets: number[];
  differences: string[];
}

/** "21:17", or a dash. */
export function scoreText(s: [number, number] | null): string {
  return s ? `${s[0]}:${s[1]}` : "–";
}

/** What reviewing a team will do, in a line. */
export function teamPlan(t: TeamMatch, sheetTeam: SheetTeam, chosen: string | null): string {
  if (chosen === null) return `Adds ${sheetTeam.name} with ${sheetTeam.players.length} players.`;
  if (chosen !== t.teamId) return "Uses the team picked; players it lacks are added.";
  const added = t.newPlayers.length;
  return added
    ? `Adds ${added} player${added === 1 ? "" : "s"} to the roster: #${t.newPlayers.join(", #")}.`
    : "Everyone on the sheet is on the roster.";
}
