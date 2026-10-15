// Made-up match for running the UI in a plain browser (`pnpm dev`) and for
// unit tests. Teams, players and codes are invented.

import type { Group, Row, Side, Snapshot } from "./snapshot";

const NAMES: Record<Side, Record<number, string>> = {
  home: {
    1: "Ardent",
    3: "Bellamy",
    4: "Corrin",
    5: "Dunmore",
    7: "Ellery",
    9: "Faraday",
    11: "Garrow",
    14: "Hollis",
    16: "Ingram",
    18: "Jessop",
  },
  visiting: {
    2: "Kestrel",
    5: "Lowther",
    6: "Marlow",
    8: "Nettleton",
    9: "Ormond",
    10: "Pryce",
    12: "Quarry",
    13: "Rook",
    15: "Stannard",
    19: "Thorne",
    21: "Upwood",
  },
};
const LIBERO: Record<Side, number> = { home: 16, visiting: 6 };

let seq = 0;
function row(text: string, kind: Row["kind"] = "touch", severity: Row["severity"] = null): Row {
  const auto = kind === "point";
  const team: Side | null = text.startsWith("*")
    ? "home"
    : text.startsWith("a")
      ? "visiting"
      : null;
  seq += 1;
  return { id: `fixture-${seq}`, index: seq - 1, text, team, kind, meaning: "", auto, severity };
}

function rally(
  n: number,
  serving: Side,
  wonBy: Side,
  after: [number, number],
  codes: string[],
): Group {
  const point = `${wonBy === "home" ? "*" : "a"}p${String(after[0]).padStart(2, "0")}:${String(after[1]).padStart(2, "0")}`;
  return {
    set: 2,
    rally: n,
    serving,
    wonBy,
    scoreAfter: { home: after[0], visiting: after[1] },
    closed: true,
    rows: [...codes.map((c) => row(c)), row(point, "point")],
  };
}

export function fixture(): Snapshot {
  seq = 0;
  const groups: Group[] = [
    rally(23, "home", "visiting", [11, 12], ["*07SM-", "a10RM+", "a15EQ+", "a02AQ#"]),
    rally(
      24,
      "visiting",
      "home",
      [12, 12],
      ["a12SH+", "*09RH!", "*01EH+", "*14AH-", "a08DH+", "a15EH#", "a05AH="],
    ),
    rally(25, "home", "home", [13, 12], ["*11SM#", "a10RM="]),
    rally(26, "home", "home", [14, 12], ["*11SM-", "a08RM="]),
    {
      set: 2,
      rally: 27,
      serving: "home",
      wonBy: null,
      scoreAfter: null,
      closed: false,
      rows: [row("*11SH+"), row("a13RM!", "touch", "warning")],
    },
  ];
  const players = (side: Side) =>
    Object.entries(NAMES[side]).map(([n, name]) => ({
      number: Number(n),
      name,
      libero: Number(n) === LIBERO[side],
    }));
  return {
    matchId: "fixture",
    teams: {
      home: { name: "Harbour City VC", players: players("home") },
      visiting: { name: "Northgate Volley", players: players("visiting") },
    },
    set: 2,
    score: { home: 14, visiting: 12 },
    serving: "home",
    sets: [
      { number: 1, score: { home: 25, visiting: 21 }, finished: true },
      { number: 2, score: { home: 14, visiting: 12 }, finished: false },
    ],
    court: { home: [11, 14, 1, 4, 7, 9], visiting: [15, 2, 5, 8, 10, 12] },
    setter: { home: 1, visiting: 15 },
    setterZone: { home: 3, visiting: 1 },
    timeouts: { home: 1, visiting: 0 },
    substitutions: { home: 2, visiting: 4 },
    groups,
    issues: [
      {
        severity: "warning",
        rule: "rotation",
        message: "visiting player 13 is not on court",
        eventId: "fixture-29",
        index: 28,
        knockOn: false,
      },
      {
        severity: "info",
        rule: "rotation",
        message: "home setter position corrected from zone 4 to 3",
        eventId: "fixture-6",
        index: 5,
        knockOn: true,
      },
    ],
    events: 412,
  };
}
