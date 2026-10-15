import { describe, expect, it } from "vitest";
import { fixture } from "./fixture";
import { setNews } from "./news";
import type { Snapshot } from "./snapshot";

function sets(snap: Snapshot, scores: [number, number][], set: number): Snapshot {
  return {
    ...snap,
    set,
    sets: scores.map(([home, visiting], i) => ({
      number: i + 1,
      score: { home, visiting },
      finished: i + 1 < set,
    })),
  };
}

describe("set news", () => {
  const base = fixture();
  const home = base.teams.home.name;

  it("says nothing when no set ended", () => {
    const a = sets(base, [[3, 2]], 1);
    expect(setNews(a, sets(base, [[4, 2]], 1))).toBe("");
  });

  it("names the set and its winner", () => {
    const before = sets(base, [[24, 20]], 1);
    const after = sets(
      base,
      [
        [25, 20],
        [0, 0],
      ],
      2,
    );
    expect(setNews(before, after)).toBe(` Set 1 to ${home} 25:20. Undo if wrong.`);
  });

  it("says when the match is over", () => {
    const before = sets(
      base,
      [
        [25, 20],
        [25, 20],
        [24, 20],
      ],
      3,
    );
    const after = {
      ...sets(
        base,
        [
          [25, 20],
          [25, 20],
          [25, 20],
        ],
        3,
      ),
      matchOver: true,
    };
    expect(setNews(before, after)).toContain("Match over");
  });

  it("asks to end a set that waits for the scout", () => {
    const before = sets(base, [[24, 20]], 1);
    const after = {
      ...sets(base, [[25, 20]], 1),
      setEndDue: true,
      score: { home: 25, visiting: 20 },
    };
    expect(setNews(before, after)).toContain("Set 1 is won at 25:20");
  });
});
