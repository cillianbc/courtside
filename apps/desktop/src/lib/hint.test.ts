import { describe, expect, it } from "vitest";
import { IDLE, hintFor } from "./hint";

const read = (text: string, meaning: string, error: string | null = null) => ({
  text,
  meaning,
  error,
});

describe("command bar hint", () => {
  it("shows the keys when the line is empty", () => {
    expect(hintFor("  ", null)).toEqual({ text: IDLE, bad: false });
  });

  it("joins the meanings of several codes", () => {
    const p = { ok: true, codes: [read("a14EH+", "set"), read("a13AH#", "attack")] };
    expect(hintFor("a14EH+ a13AH#", p)).toEqual({ text: "set · attack", bad: false });
  });

  it("names the code that is wrong when there are several", () => {
    const p = { ok: false, codes: [read("*T", "Timeout"), read("x1", "", "Start with *")] };
    expect(hintFor("*T x1", p)).toEqual({ text: "x1: Start with *", bad: true });
    expect(hintFor("x1", { ok: false, codes: [read("x1", "", "Start with *")] })).toEqual({
      text: "Start with *",
      bad: true,
    });
  });
});
