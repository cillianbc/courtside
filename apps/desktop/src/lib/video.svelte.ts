// The video player as the UI sees it: its last known state and the
// transport commands. The panel's buttons and the keymap run the same code.

import * as backend from "./backend";
import type { PlayerState } from "./backend";
import type { Clip } from "./snapshot";

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

/** How far past a clip the video may be and still count as playing it. */
export const STRAY_MS = 2000;

/**
 * What a playlist does with the video at `ms` while `clip` plays: keep
 * playing it, move on (its end is reached), or let go (the scout moved the
 * video somewhere else).
 */
export function clipCue(clip: Clip, ms: number): "play" | "next" | "left" {
  if (ms < clip.startMs - STRAY_MS || ms >= clip.endMs + STRAY_MS) return "left";
  return ms >= clip.endMs ? "next" : "play";
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
    this.#followPlaylist();
  }

  /** The clip playlist and the clip playing (-1: none, the video plays freely). */
  playlist = $state<Clip[]>([]);
  clipAt = $state(-1);
  /**
   * When the last clip was cued (`performance.now()`), or `Infinity` while
   * it is: a poll sent before the seek can land after it, with the old time.
   */
  #cuedAt = 0;

  get clip(): Clip | null {
    return this.playlist[this.clipAt] ?? null;
  }

  /** Plays clip `i` of the playlist from its start; past the end, stops. */
  playClip = async (i: number) => {
    const clip = this.playlist[i];
    if (!clip || !this.open) {
      this.clipAt = -1;
      return;
    }
    this.clipAt = i;
    this.#cuedAt = Infinity;
    try {
      await this.seekTo(clip.startMs / 1000);
      await this.run(() => backend.videoPause(false));
    } finally {
      this.#cuedAt = performance.now();
    }
  };

  /** Moves `by` clips (from the first, when none plays). */
  stepClip = (by: number) => {
    if (!this.playlist.length) return;
    const at = this.clipAt < 0 ? (by > 0 ? -1 : this.playlist.length) : this.clipAt;
    void this.playClip(Math.max(0, Math.min(this.playlist.length - 1, at + by)));
  };

  stopClips = () => {
    this.clipAt = -1;
  };

  /** Moves on at a clip's end; pauses after the last. */
  #followPlaylist() {
    const p = this.#player;
    const clip = this.clip;
    const settling = performance.now() - this.#cuedAt < 500;
    if (!clip || settling || !p || p.paused || p.time === null) return;
    const cue = clipCue(clip, p.time * 1000);
    if (cue === "left") this.clipAt = -1;
    else if (cue === "next") {
      if (this.clipAt + 1 < this.playlist.length) void this.playClip(this.clipAt + 1);
      else {
        this.clipAt = -1;
        void this.run(() => backend.videoPause(true));
      }
    }
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
