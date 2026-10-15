// Season settings (mirrors courtside_store::Settings) and the keymap.

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
  keymap: KeyBinding[];
  rules: {
    /** Substitutions per team and set. */
    substitutions: number;
    liberoMayServe: boolean;
    /** Hold each set end until the scout confirms it. */
    confirmSetEnd: boolean;
  };
}

export interface KeyBinding {
  action: string;
  keys: string;
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
  { id: "pointLeft", label: "Point to the team on the left", keys: "Alt+ArrowLeft" },
  { id: "pointRight", label: "Point to the team on the right", keys: "Alt+ArrowRight" },
  { id: "timeoutHome", label: "Timeout, home", keys: "Alt+T" },
  { id: "timeoutAway", label: "Timeout, away", keys: "Alt+Shift+T" },
  { id: "subHome", label: "Substitution, home", keys: "Alt+S" },
  { id: "subAway", label: "Substitution, away", keys: "Alt+Shift+S" },
  { id: "rotateHome", label: "Rotate home forward", keys: "Alt+R" },
  { id: "rotateAway", label: "Rotate away forward", keys: "Alt+Shift+R" },
  { id: "swapSides", label: "Swap court sides", keys: "Alt+W" },
  { id: "endView", label: "Side or end view", keys: "Alt+V" },
  { id: "exportDvw", label: "Export .dvw", keys: "Mod+E" },
];

/** The season's keymap, with defaults for actions it doesn't list. */
export function effectiveKeymap(saved: KeyBinding[]): KeyBinding[] {
  return ACTIONS.map((a) => saved.find((k) => k.action === a.id) ?? { action: a.id, keys: a.keys });
}

/**
 * A key press as a binding string: modifiers in a fixed order, then the
 * key. Letters and digits come from the physical key, so Alt combinations
 * on a Mac (which type symbols) still read as `Alt+T`. `null` for a bare
 * modifier press.
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
  else if (key.length === 1) key = key.toUpperCase();
  const parts: string[] = [];
  if (e.ctrlKey || e.metaKey) parts.push("Mod");
  if (e.altKey) parts.push("Alt");
  if (e.shiftKey) parts.push("Shift");
  parts.push(key);
  return parts.join("+");
}

/** The action bound to `keys`, if any. */
export function actionFor(keymap: KeyBinding[], keys: string | null): string | null {
  if (!keys) return null;
  return keymap.find((k) => k.keys === keys)?.action ?? null;
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
