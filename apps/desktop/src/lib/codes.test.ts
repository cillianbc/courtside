import { describe, expect, it } from "vitest";
import { parseSix, pointCode, rotationCode, substitutionCode, timeoutCode } from "./codes";

describe("button codes", () => {
  it("writes the point with the score after it", () => {
    expect(pointCode("home", { home: 24, visiting: 9 })).toBe("*p25:09");
    expect(pointCode("visiting", { home: 3, visiting: 4 })).toBe("ap03:05");
  });

  it("writes timeouts and substitutions", () => {
    expect(timeoutCode("visiting")).toBe("aT");
    expect(substitutionCode("home", 6, 17)).toBe("*c06:17");
  });

  it("rotates the setter zone with wrap-around", () => {
    expect(rotationCode("home", 1, true)).toBe("*z6");
    expect(rotationCode("home", 2, true)).toBe("*z1");
    expect(rotationCode("visiting", 6, false)).toBe("az1");
    expect(rotationCode("visiting", 3, false)).toBe("az4");
    expect(rotationCode("home", null, true)).toBeNull();
  });
});

describe("parseSix", () => {
  it("reads six numbers", () => {
    expect(parseSix(" 8 12,13 11  3 14 ")).toEqual({ court: [8, 12, 13, 11, 3, 14] });
  });

  it("explains what is wrong", () => {
    expect(parseSix("8 12 13")).toEqual({ error: "Six numbers for zones 1–6, got 3." });
    expect(parseSix("8 12 13 x 3 14")).toEqual({ error: "x isn't a shirt number." });
  });
});
