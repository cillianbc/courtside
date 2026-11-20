import { describe, expect, it } from "vitest";
import { cells, rotationName, rotationOf, type RotationView } from "./rotation";
import { fixture } from "./fixture";

const view: RotationView = {
  set: 2,
  score: { home: 5, visiting: 3 },
  serving: "visiting",
  court: { home: [7, 1, 9, 4, 11, 14], visiting: [15, 2, null, 8, 10, 12] },
  setter: { home: 1, visiting: 15 },
  setterZone: { home: 2, visiting: 1 },
  liberos: { home: [5], visiting: [] },
};

describe("rotation", () => {
  it("lays the six out by zone as the court does", () => {
    // Net on the right: back row on the left, zone 4 at the top.
    expect(cells(view, "home", "right").map((c) => c.number)).toEqual([11, 4, 14, 9, 7, 1]);
    // Net at the top: front row 4, 3, 2 first.
    expect(cells(view, "home", "top").map((c) => c.zone)).toEqual([4, 3, 2, 5, 6, 1]);
  });

  it("marks the setter and the serving team's zone 1", () => {
    const home = cells(view, "home", "left");
    expect(home.filter((c) => c.setter).map((c) => c.zone)).toEqual([2]);
    expect(home.some((c) => c.server)).toBe(false);
    const away = cells(view, "visiting", "left");
    expect(away.filter((c) => c.server).map((c) => c.number)).toEqual([15]);
    expect(away.find((c) => c.zone === 1)?.setter).toBe(true);
    // An empty zone is never the setter.
    expect(away.find((c) => c.zone === 3)).toMatchObject({ number: null, setter: false });
  });

  it("names the setter's rotation", () => {
    expect(rotationName(view.setterZone.home)).toBe("P2");
    expect(rotationName(null)).toBe("P?");
    expect(rotationName(7)).toBe("P?");
  });

  it("reads the end of the log from the snapshot", () => {
    const snap = fixture();
    const r = rotationOf(snap);
    expect(r.court).toEqual(snap.court);
    expect(r.serving).toBe(snap.serving);
    expect(r.liberos).toEqual(snap.liberos);
  });
});
