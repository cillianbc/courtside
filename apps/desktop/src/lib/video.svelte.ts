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

/**
 * The video time at the first keystroke of each code on the command line,
 * kept as the line changes: a code typed anew gets `now`, codes already
 * there keep theirs.
 */
export function stampTokens(
  text: string,
  times: (number | null)[],
  now: number | null,
): (number | null)[] {
  const n = text.trim() ? text.trim().split(/\s+/).length : 0;
  const out = times.slice(0, n);
  while (out.length < n) out.push(now);
  return out;
}

class Video {
  #player = $state<PlayerState | null>(null);
  /** When `player` was last read (`performance.now()`). */
  #readAt = 0;
  error = $state("");

  get player(): PlayerState | null {
    return this.#player;
  }

  set player(s: PlayerState | null) {
    this.#player = s;
    this.#readAt = performance.now();
  }

  /** The video time now in ms: the last state read, run on while playing. */
  nowMs(): number | null {
    const p = this.#player;
    if (!p?.path || p.time === null) return null;
    const run = p.paused ? 0 : ((performance.now() - this.#readAt) / 1000) * p.speed;
    return Math.round((p.time + run) * 1000);
  }

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
