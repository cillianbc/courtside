// The Video layout's rotation (#74): who is on court for each team, at the
// picked code (from the backend) or at the end of the log (the snapshot).

import { LAYOUT, type Net } from "./court";
import type { Pair, Side, Snapshot } from "./snapshot";

/** Who was on court at one code (mirrors `courtside_console::snapshot::RotationView`). */
export interface RotationView {
  set: number;
  score: Pair<number>;
  serving: Side | null;
  /** Shirt numbers in zones 1–6. */
  court: Pair<(number | null)[]>;
  setter: Pair<number | null>;
  setterZone: Pair<number | null>;
  liberos: Pair<number[]>;
}

/** The rotation at the end of the log, from the snapshot. */
export function rotationOf(snap: Snapshot): RotationView {
  return {
    set: snap.set,
    score: snap.score,
    serving: snap.serving,
    court: snap.court,
    setter: snap.setter,
    setterZone: snap.setterZone,
    liberos: snap.liberos,
  };
}

/** One zone of a half court as drawn. */
export interface Cell {
  zone: number;
  number: number | null;
  setter: boolean;
  /** The serving team's player in zone 1. */
  server: boolean;
}

/** `side`'s zones in reading order for a half court with its net at `net`. */
export function cells(view: RotationView, side: Side, net: Net): Cell[] {
  return LAYOUT[net].map((zone) => {
    const number = view.court[side][zone - 1] ?? null;
    return {
      zone,
      number,
      setter: number !== null && number === view.setter[side],
      server: view.serving === side && zone === 1,
    };
  });
}

/** The setter's rotation as scouts name it: P1–P6 for the setter's zone. */
export function rotationName(setterZone: number | null): string {
  return setterZone !== null && setterZone >= 1 && setterZone <= 6 ? `P${setterZone}` : "P?";
}
