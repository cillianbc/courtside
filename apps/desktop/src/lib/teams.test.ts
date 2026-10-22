import { describe, expect, it } from "vitest";
import {
  blankPlayer,
  describeImport,
  guessPairs,
  nextNumber,
  rosterProblems,
  suggestCode,
  TEAM_COLOURS,
  teamColour,
} from "./teams";

const p = (number: number, lastName: string, firstName = "", code = "") => ({
  ...blankPlayer(number),
  lastName,
  firstName,
  code,
  id: `${lastName}-${number}`,
});

describe("player codes", () => {
  it("matches the season's rule", () => {
    expect(suggestCode("Smith", "John", 4)).toBe("SMI-JOH");
    expect(suggestCode("Ó Sé", "", 7)).toBe("ÓSÉ");
    expect(suggestCode("", "", 12)).toBe("P12");
  });
});

describe("roster checks", () => {
  it("flags duplicate numbers, codes and missing names", () => {
    const rows = [
      p(1, "Ardent"),
      p(1, "Bellamy"),
      p(2, ""),
      p(3, "Corrin", "", "X"),
      p(4, "Dunmore", "", "X"),
    ];
    const problems = rosterProblems(rows);
    expect([...problems.entries()]).toEqual([
      [1, "#1 is already on the roster."],
      [2, "Enter a last name."],
      [4, "Code X is already on the roster."],
    ]);
    expect(rosterProblems([p(100, "A")]).get(0)).toBe("Shirt numbers go from 0 to 99.");
  });

  it("picks the next free number", () => {
    expect(nextNumber([])).toBe(1);
    expect(nextNumber([p(4, "A"), p(7, "B")])).toBe(8);
  });
});

describe("merge pairs", () => {
  it("pairs by code, then name, then number and last name", () => {
    const from = [
      p(1, "Ardent", "Mara", "ARD"),
      p(2, "Bellamy", "Jo"),
      p(3, "Corrin"),
      p(4, "Nobody"),
    ];
    const into = [p(9, "Ardent", "M", "ARD"), p(5, "Bellamy", "Jo"), p(3, "Corrin"), p(4, "Other")];
    expect([...guessPairs(from, into)]).toEqual([
      ["Ardent-1", "Ardent-9"],
      ["Bellamy-2", "Bellamy-5"],
      ["Corrin-3", "Corrin-3"],
    ]);
  });
});

describe("team import summary", () => {
  it("says what happened", () => {
    const r = {
      teamId: "t",
      teamName: "Northgate",
      created: true,
      playersAdded: 1,
      playersKept: 0,
    };
    expect(describeImport(r)).toBe("Added Northgate with 1 player.");
    expect(describeImport({ ...r, created: false, playersAdded: 2, playersKept: 12 })).toBe(
      "Northgate was already here: added 2 players, kept 12 players as they were.",
    );
  });
});

describe("team colours", () => {
  // WCAG contrast between two #rrggbb colours.
  const lum = (hex: string) => {
    const [r, g, b] = [1, 3, 5].map((i) => {
      const c = parseInt(hex.slice(i, i + 2), 16) / 255;
      return c <= 0.03928 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4;
    });
    return 0.2126 * (r ?? 0) + 0.7152 * (g ?? 0) + 0.0722 * (b ?? 0);
  };
  const ratio = (a: string, b: string) => {
    const [x, y] = [lum(a), lum(b)].sort((p, q) => q - p);
    return ((x ?? 0) + 0.05) / ((y ?? 0) + 0.05);
  };

  it("every preset reads on the panels in light and dark (v6 T13)", () => {
    for (const c of TEAM_COLOURS) {
      expect(ratio(c.light, "#ffffff"), c.key).toBeGreaterThanOrEqual(4.5); // light --panel
      expect(ratio(c.light, "#e6ebf1"), c.key).toBeGreaterThanOrEqual(4.5); // light --sunk
      expect(ratio(c.dark, "#152130"), c.key).toBeGreaterThanOrEqual(4.5); // dark --panel
    }
  });

  it("finds a preset by its key", () => {
    expect(teamColour("navy")?.light).toBe("#2b3f8f");
    expect(teamColour("")).toBeNull();
  });
});
