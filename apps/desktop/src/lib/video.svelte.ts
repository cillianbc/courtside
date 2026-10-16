// The video player as the UI sees it: its last known state and the
// transport commands. The panel's buttons and the keymap run the same code.

import * as backend from "./backend";
import type { PlayerState } from "./backend";

/** Playback speeds the speed button and keys step through. */
export const SPEEDS = [0.25, 0.5, 1, 1.5, 2];
/** Seconds a jump moves. */
export const JUMP = 5;

/** The speed `by` steps from `speed` (clamped, no wrap). */
export function stepSpeed(speed: number, by: number): number {
  const i = SPEEDS.indexOf(speed);
  const at = i < 0 ? SPEEDS.indexOf(1) : i;
  return SPEEDS[Math.max(0, Math.min(SPEEDS.length - 1, at + by))] ?? 1;
}

class Video {
  player = $state<PlayerState | null>(null);
  error = $state("");

  get open(): boolean {
    return !!this.player?.path;
  }

  async run(f: () => Promise<PlayerState | null>): Promise<void> {
    this.error = "";
    try {
      this.player = (await f()) ?? this.player;
    } catch (e) {
      this.error = String(e);
    }
  }

  togglePlay = () => this.run(() => backend.videoPause(!(this.player?.paused ?? true)));
  seek = (by: number) =>
    this.run(() => backend.videoCommand(["seek", String(by), "relative+exact"]));
  seekTo = (at: number) =>
    this.run(() => backend.videoCommand(["seek", at.toFixed(3), "absolute+exact"]));
  step = (back: boolean) =>
    this.run(() => backend.videoCommand([back ? "frame-back-step" : "frame-step"]));
  /** Steps the speed; `wrap` cycles back to the slowest after the fastest (the button). */
  speed = (by: number, wrap = false) =>
    this.run(() => {
      const now = this.player?.speed ?? 1;
      const i = SPEEDS.indexOf(now);
      const next = wrap && i === SPEEDS.length - 1 ? (SPEEDS[0] ?? 1) : stepSpeed(now, by);
      return backend.videoSpeed(next);
    });
}

export const video = new Video();
