import { keyFor } from "./settings";
import { ui } from "./ui.svelte";

/** `label` with the key bound to `action` on this computer, for a tooltip: "Undo (⌘Z)". */
export function withKey(label: string, action: string): string {
  const key = keyFor(ui.keymap, action);
  return key ? `${label} (${key})` : label;
}
