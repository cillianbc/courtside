import type { Snapshot } from "./snapshot";
import { ui } from "./ui.svelte";

/** Shows a code in the stream: its set, its rally opened, scrolled to and flashed. */
export function showCode(snap: Snapshot, eventId: string) {
  const group = snap.groups.find((g) => g.rows.some((r) => r.id === eventId));
  if (group) ui.shownSet = group.set === snap.set ? null : group.set;
  requestAnimationFrame(() => {
    const el = document.getElementById(`ev-${eventId}`);
    if (!el) return;
    el.closest(".rally")?.classList.remove("closed");
    el.scrollIntoView({ block: "center" });
    el.classList.remove("hl");
    void el.offsetWidth;
    el.classList.add("hl");
  });
}
