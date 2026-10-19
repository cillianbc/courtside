// Season settings (mirrors courtside_store::Settings) and this computer's
// keys (courtside_console::Keys).

/** The keys of the computer the scout codes on, not of a season. */
export interface Keys {
  keymap: KeyBinding[];
  /** Keys the command line types other text for (`]` → `#`). */
  keyText: KeyText[];
}

export interface Settings {
  typing: {
    skill: string;
    /** Per skill in SKILLS order. */
    types: string[];
    evaluations: string[];
    predict: boolean;
  };
  tables: {
    winning: { skill: string; lose: string; win: string }[];
    combinations: {
      code: string;
      startZone: number;
      tempo: string;
      target: string;
      description: string;
    }[];
    setterCalls: { code: string; description: string }[];
  };
  rules: {
    /** Substitutions per team and set. */
    substitutions: number;
    liberoMayServe: boolean;
    /** Hold each set end until the scout confirms it. */
    confirmSetEnd: boolean;
  };
  video: {
    /**
     * Predictive: a rally's later touches follow its first by `stepMs`
     * each. Exact: each code keeps its own first keystroke's video time.
     */
    predictive: boolean;
    stepMs: number;
    /** Video a clip shows before and after its code. */
    preRollMs: number;
    postRollMs: number;
  };
}

export interface KeyBinding {
  action: string;
  keys: string;
}

/** A key that types `text` on the command line instead of its own character. */
export interface KeyText {
  keys: string;
  text: string;
}

export const SKILLS = ["S", "R", "E", "A", "B", "D", "F"] as const;
export const SKILL_NAMES: Record<string, string> = {
  S: "Serve",
  R: "Reception",
  E: "Set",
  A: "Attack",
  B: "Block",
  D: "Dig",
  F: "Freeball",
};
export const TYPES = ["H", "M", "Q", "T", "U", "N", "O"] as const;
export const EVALUATIONS = ["#", "+", "!", "-", "/", "="] as const;

/** Console actions that can have a key. Keys are Courtside's own defaults. */
export const ACTIONS: readonly { id: string; label: string; keys: string }[] = [
  { id: "undo", label: "Undo", keys: "Mod+Z" },
  { id: "redo", label: "Redo", keys: "Mod+Shift+Z" },
  { id: "focusCode", label: "Go to the command line", keys: "Mod+K" },
  // Not Alt+arrows: on a Mac those move by word in the command line.
  { id: "pointLeft", label: "Point to the team on the left", keys: "Alt+1" },
  { id: "pointRight", label: "Point to the team on the right", keys: "Alt+2" },
  { id: "timeoutHome", label: "Timeout, home", keys: "Alt+T" },
  { id: "timeoutAway", label: "Timeout, away", keys: "Alt+Shift+T" },
  { id: "subHome", label: "Substitution, home", keys: "Alt+S" },
  { id: "subAway", label: "Substitution, away", keys: "Alt+Shift+S" },
  { id: "rotateHome", label: "Rotate home forward", keys: "Alt+R" },
  { id: "rotateAway", label: "Rotate away forward", keys: "Alt+Shift+R" },
  { id: "swapSides", label: "Swap court sides", keys: "Alt+W" },
  { id: "endView", label: "Side or end view", keys: "Alt+V" },
  { id: "layout", label: "Coding or video layout", keys: "Alt+Shift+V" },
  { id: "exportDvw", label: "Export .dvw", keys: "Mod+E" },
  // Stepping through codes to mark where each play went.
  { id: "codePrev", label: "Pick the previous play", keys: "Alt+Shift+[" },
  { id: "codeNext", label: "Pick the next play", keys: "Alt+Shift+]" },
  // Video transport. Alt so they work while typing a code.
  { id: "videoPlay", label: "Video: play or pause", keys: "Alt+K" },
  { id: "videoBack", label: "Video: back 5 seconds", keys: "Alt+J" },
  { id: "videoForward", label: "Video: forward 5 seconds", keys: "Alt+L" },
  { id: "videoBackLong", label: "Video: back 30 seconds", keys: "Alt+Shift+J" },
  { id: "videoForwardLong", label: "Video: forward 30 seconds", keys: "Alt+Shift+L" },
  { id: "videoFrameBack", label: "Video: one frame back", keys: "Alt+," },
  { id: "videoFrameForward", label: "Video: one frame on", keys: "Alt+." },
  { id: "videoSlower", label: "Video: slower", keys: "Alt+Shift+," },
  { id: "videoFaster", label: "Video: faster", keys: "Alt+Shift+." },
  { id: "clipPrev", label: "Clips: previous clip", keys: "Alt+[" },
  { id: "clipNext", label: "Clips: next clip", keys: "Alt+]" },
];

/**
 * Keys named by their physical place, as on a US layout: a Mac's Option
 * (and Shift) change what these keys type, not which key was pressed.
 */
const PHYSICAL: Record<string, string> = {
  Comma: ",",
  Period: ".",
  Slash: "/",
  Semicolon: ";",
  Quote: "'",
  BracketLeft: "[",
  BracketRight: "]",
  Backslash: "\\",
  Minus: "-",
  Equal: "=",
  Backquote: "`",
  Space: "Space",
};

/** The season's keymap, with defaults for actions it doesn't list. */
export function effectiveKeymap(saved: KeyBinding[]): KeyBinding[] {
  return ACTIONS.map((a) => saved.find((k) => k.action === a.id) ?? { action: a.id, keys: a.keys });
}

/**
 * A key press as a binding string: modifiers in a fixed order, then the
 * key. Letters, digits and punctuation come from the physical key, so Alt
 * combinations on a Mac (which type symbols) still read as `Alt+T` or
 * `Alt+,`. `null` for a bare modifier press.
 */
export function keyString(e: {
  key: string;
  code: string;
  ctrlKey: boolean;
  metaKey: boolean;
  altKey: boolean;
  shiftKey: boolean;
}): string | null {
  if (["Control", "Meta", "Alt", "Shift"].includes(e.key)) return null;
  let key = e.key;
  if (/^Key[A-Z]$/.test(e.code)) key = e.code.slice(3);
  else if (/^Digit[0-9]$/.test(e.code)) key = e.code.slice(5);
  else if (PHYSICAL[e.code]) key = PHYSICAL[e.code] ?? key;
  else if (key.length === 1) key = key.toUpperCase();
  const parts: string[] = [];
  if (e.ctrlKey || e.metaKey) parts.push("Mod");
  if (e.altKey) parts.push("Alt");
  if (e.shiftKey) parts.push("Shift");
  parts.push(key);
  return parts.join("+");
}

/** Whether this computer is a Mac, whose keyboards name keys differently. */
export const isMac =
  typeof navigator !== "undefined" && /Mac|iPhone|iPad/.test(navigator.userAgent);

/** Mac modifier symbols, in the order a Mac's menus list them. */
const MAC_MODIFIERS: [string, string][] = [
  ["Alt", "⌥"],
  ["Shift", "⇧"],
  ["Mod", "⌘"],
];

/**
 * A binding as this computer's keyboard names it: `Alt+Shift+J` is ⌥⇧J on
 * a Mac and Alt+Shift+J elsewhere; `Mod` is ⌘ on a Mac and Ctrl elsewhere.
 * Bindings stay saved as `Mod`/`Alt`, the same on every computer.
 */
export function keyLabel(keys: string, mac = isMac): string {
  if (!keys) return "";
  const mods = keys.split("+");
  const key = mods.pop() ?? "";
  if (mac) {
    return (
      MAC_MODIFIERS.filter(([m]) => mods.includes(m))
        .map(([, sym]) => sym)
        .join("") + key
    );
  }
  return [...mods.map((m) => (m === "Mod" ? "Ctrl" : m)), key].join("+");
}

/** The key bound to `action`, as this computer names it; "" if none. */
export function keyFor(keymap: KeyBinding[], action: string, mac = isMac): string {
  return keyLabel(keymap.find((k) => k.action === action)?.keys ?? "", mac);
}

/** What this computer calls Mod and Alt, for help text. */
export const MOD_NAME = isMac ? "⌘ Cmd" : "Ctrl";
export const ALT_NAME = isMac ? "⌥ Option" : "Alt";

/** The action bound to `keys`, if any. */
export function actionFor(keymap: KeyBinding[], keys: string | null): string | null {
  if (!keys) return null;
  return keymap.find((k) => k.keys === keys)?.action ?? null;
}

/** The text a remapped key types, if it is one. */
export function textFor(keyText: KeyText[], keys: string | null): string | null {
  if (!keys) return null;
  return keyText.find((k) => k.keys === keys && k.text)?.text ?? null;
}

/**
 * `value` with `text` typed over the selection `start`–`end`, and where
 * the caret goes after it.
 */
export function typeText(
  value: string,
  start: number,
  end: number,
  text: string,
): { value: string; caret: number } {
  return { value: value.slice(0, start) + text + value.slice(end), caret: start + text.length };
}

/** Keys bound to more than one action. */
export function conflicts(keymap: KeyBinding[]): Set<string> {
  const seen = new Map<string, number>();
  for (const k of keymap) if (k.keys) seen.set(k.keys, (seen.get(k.keys) ?? 0) + 1);
  return new Set([...seen].filter(([, n]) => n > 1).map(([k]) => k));
}

/**
 * Whether a binding is safe while the scout types on the command line: it
 * must use Mod or Alt, or be a function key, so plain typing never
 * triggers it.
 */
export function usableWhileTyping(keys: string): boolean {
  return /(^|\+)(Mod|Alt)\+/.test(keys) || /(^|\+)F([1-9]|1[0-2])$/.test(keys);
}
