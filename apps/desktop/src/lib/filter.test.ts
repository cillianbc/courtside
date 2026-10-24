import { describe, expect, it } from "vitest";
import { emptyFilter, hasFields, stepMatch } from "./filter";

describe("search in the code log", () => {
  const order = ["a", "b", "c", "d", "e"];
  const found = ["b", "d"];

  it("steps from match to match and round the end", () => {
    expect(stepMatch(found, order, "b", 1)).toBe("d");
    expect(stepMatch(found, order, "d", 1)).toBe("b");
    expect(stepMatch(found, order, "b", -1)).toBe("d");
  });

  it("starts from the picked code when it isn't a match", () => {
    expect(stepMatch(found, order, "c", 1)).toBe("d");
    expect(stepMatch(found, order, "c", -1)).toBe("b");
    expect(stepMatch(found, order, "e", 1)).toBe("b");
  });

  it("starts at an end with nothing picked, and finds nothing in nothing", () => {
    expect(stepMatch(found, order, null, 1)).toBe("b");
    expect(stepMatch(found, order, null, -1)).toBe("d");
    expect(stepMatch([], order, "a", 1)).toBeNull();
  });

  it("tells the code text from the other fields", () => {
    expect(hasFields({ ...emptyFilter(), text: "D=" })).toBe(false);
    expect(hasFields({ ...emptyFilter(), skill: "D" })).toBe(true);
  });
});
