import type { Snapshot } from "./snapshot";
import { ui } from "./ui.svelte";
import { video } from "./video.svelte";

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

/** Picks a code: shown in the stream, and in the video where it has a video time. */
export function pickCode(snap: Snapshot, eventId: string) {
  ui.selected = eventId;
  showCode(snap, eventId);
  const r = snap.groups.flatMap((g) => g.rows).find((x) => x.id === eventId);
  if (r && r.videoMs !== null && video.open) void video.seekTo(r.videoMs / 1000);
}
