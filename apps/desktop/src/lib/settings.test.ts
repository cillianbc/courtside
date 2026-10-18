import { describe, expect, it } from "vitest";
import {
  ACTIONS,
  actionFor,
  conflicts,
  effectiveKeymap,
  keyString,
  textFor,
  typeText,
  usableWhileTyping,
} from "./settings";

const press = (over: Partial<Parameters<typeof keyString>[0]>) =>
  keyString({
    key: "",
    code: "",
    ctrlKey: false,
    metaKey: false,
    altKey: false,
    shiftKey: false,
    ...over,
  });

describe("keymap", () => {
  it("reads key presses in a fixed order", () => {
    expect(press({ key: "z", code: "KeyZ", ctrlKey: true, shiftKey: true })).toBe("Mod+Shift+Z");
    expect(press({ key: "Z", code: "KeyZ", metaKey: true })).toBe("Mod+Z");
    // Option+T types a dagger on a Mac; the physical key still counts.
    expect(press({ key: "†", code: "KeyT", altKey: true })).toBe("Alt+T");
    expect(press({ key: "F5", code: "F5" })).toBe("F5");
    expect(press({ key: "Shift", code: "ShiftLeft", shiftKey: true })).toBeNull();
    // Option+comma types ≤, Shift+period types >: the key is still the same.
    expect(press({ key: "≤", code: "Comma", altKey: true })).toBe("Alt+,");
    expect(press({ key: "˘", code: "Period", altKey: true, shiftKey: true })).toBe("Alt+Shift+.");
    expect(press({ key: " ", code: "Space", altKey: true })).toBe("Alt+Space");
  });

  it("fills in defaults and finds actions", () => {
    const km = effectiveKeymap([{ action: "undo", keys: "F8" }]);
    expect(km).toHaveLength(ACTIONS.length);
    expect(actionFor(km, "F8")).toBe("undo");
    expect(actionFor(km, "Mod+Z")).toBeNull();
    expect(actionFor(km, "Alt+T")).toBe("timeoutHome");
  });

  it("finds conflicts and keys unsafe while typing", () => {
    const km = effectiveKeymap([{ action: "redo", keys: "Mod+Z" }]);
    expect([...conflicts(km)]).toEqual(["Mod+Z"]);
    expect(conflicts(effectiveKeymap([])).size).toBe(0);
    expect(ACTIONS.every((a) => usableWhileTyping(a.keys))).toBe(true);
    expect(usableWhileTyping("Alt+T")).toBe(true);
    expect(usableWhileTyping("F5")).toBe(true);
    expect(usableWhileTyping("Shift+T")).toBe(false);
    expect(usableWhileTyping("T")).toBe(false);
  });
});

describe("keys that type", () => {
  const remap = [
    { keys: "]", text: "#" },
    { keys: "[", text: "+" },
  ];

  it("finds the text for the physical key", () => {
    expect(textFor(remap, press({ key: "]", code: "BracketRight" }))).toBe("#");
    expect(textFor(remap, press({ key: "[", code: "BracketLeft" }))).toBe("+");
    expect(textFor(remap, press({ key: "}", code: "BracketRight", shiftKey: true }))).toBeNull();
    expect(textFor(remap, press({ key: "8", code: "Digit8" }))).toBeNull();
  });

  it("types over the selection", () => {
    expect(typeText("8A", 2, 2, "#")).toEqual({ value: "8A#", caret: 3 });
    expect(typeText("8A+", 2, 3, "#")).toEqual({ value: "8A#", caret: 3 });
    expect(typeText("8 9", 1, 1, "K1")).toEqual({ value: "8K1 9", caret: 3 });
  });
});
