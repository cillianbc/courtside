import { describe, expect, it } from "vitest";
import { blankPlayer, guessPairs, nextNumber, rosterProblems, suggestCode } from "./teams";

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
