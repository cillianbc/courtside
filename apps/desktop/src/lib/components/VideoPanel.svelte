<script lang="ts">
  import { open } from "@tauri-apps/plugin-dialog";
  import { onDestroy, onMount } from "svelte";
  import * as backend from "../backend";
  import { ui } from "../ui.svelte";
  import { JUMP, video } from "../video.svelte";
  import Icon from "./Icon.svelte";

  // libmpv draws in a native view the backend keeps over `screen`; this
  // panel tells it where the screen is and drives playback.
  let screen: HTMLDivElement | undefined = $state();
  const player = $derived(video.player);

  const VIDEO = [
    { name: "Video", extensions: ["mp4", "mov", "m4v", "mkv", "avi", "mts", "m2ts", "webm", "ts"] },
  ];

  function bounds(): backend.Bounds {
    const r = screen?.getBoundingClientRect();
    return { x: r?.left ?? 0, y: r?.top ?? 0, width: r?.width ?? 0, height: r?.height ?? 0 };
  }

  const place = () => void backend.videoPlace(bounds(), ui.covering === 0).catch(() => {});
  // Hidden while a menu or dialog is open over the window.
  $effect(() => {
    void ui.covering;
    if (player?.path) place();
  });

  // The match's video (from its file, or opened with it before).
  const linked = $derived(ui.snapshot?.videoPath ?? null);
  const fileName = (p: string) => p.split(/[\\/]/).pop() ?? p;

  /** Opens `known`, or a video picked in a dialog; the match remembers it. */
  const openVideo = (known?: string) =>
    video.run(async () => {
      const path = known ?? (await open({ filters: VIDEO }));
      if (typeof path !== "string") return null;
      const state = await backend.videoOpen(path, bounds());
      if (path !== linked) ui.snapshot = await backend.matchVideoSet(path);
      return state;
    });

  function scrubTo(e: MouseEvent) {
    const d = player?.duration;
    const bar = e.currentTarget as HTMLElement;
    if (!d) return;
    const r = bar.getBoundingClientRect();
    void video.seekTo(Math.max(0, Math.min(1, (e.clientX - r.left) / r.width)) * d);
  }

  // Sync on the frame shown: the first serve is here.
  function syncHere() {
    const at = video.nowMs();
    if (at === null) return;
    void ui.apply(`Synced: the first serve is at ${timecode(at / 1000, player?.fps ?? null)}`, () =>
      backend.videoSync(at),
    );
  }

  /** `m:ss.ff`, frames when the frame rate is known. */
  function timecode(t: number | null, fps: number | null): string {
    if (t === null) return "--:--";
    const m = Math.floor(t / 60);
    const s = Math.floor(t % 60);
    const base = `${m}:${String(s).padStart(2, "0")}`;
    if (!fps) return base;
    const f = Math.floor((t % 1) * fps);
    return `${base}.${String(f).padStart(2, "0")}`;
  }

  let poll: ReturnType<typeof setInterval> | undefined;
  let observer: ResizeObserver | undefined;
  onMount(() => {
    observer = new ResizeObserver(place);
    if (screen) observer.observe(screen);
    window.addEventListener("resize", place);
    poll = setInterval(() => {
      if (player?.path)
        void backend.videoState().then(
          (s) => s && (video.player = s),
          () => {},
        );
    }, 200);
    // A video opened before (another screen in between) shows again.
    void backend.videoState().then(
      (s) => {
        if (s?.path) {
          video.player = s;
          place();
        }
      },
      () => {},
    );
  });
  onDestroy(() => {
    observer?.disconnect();
    window.removeEventListener("resize", place);
    clearInterval(poll);
    void backend.videoPlace(bounds(), false).catch(() => {});
  });

  const progress = $derived(
    player?.time != null && player.duration ? (player.time / player.duration) * 100 : 0,
  );
</script>

<section class="video" aria-label="Video">
  <div class="screen" bind:this={screen}>
    {#if !player?.path}
      <div>
        <svg
          viewBox="0 0 180 90"
          fill="none"
          stroke="currentColor"
          stroke-width="1.5"
          aria-hidden="true"
          ><rect x="10" y="15" width="160" height="60" rx="2" /><path
            d="M90 8v74M63 15v60M117 15v60"
          /><circle cx="90" cy="8" r="2" /><circle cx="90" cy="82" r="2" /></svg
        >
        {#if linked}
          <p>Match video: {fileName(linked)}</p>
          <button
            class="btn"
            type="button"
            disabled={!backend.inTauri}
            title={linked}
            onclick={() => openVideo(linked)}>Open it</button
          >
          <button class="btn" type="button" disabled={!backend.inTauri} onclick={() => openVideo()}
            >Another…</button
          >
        {:else}
          <p>No match video.</p>
          <button class="btn" type="button" disabled={!backend.inTauri} onclick={() => openVideo()}
            >Open video</button
          >
        {/if}
        {#if video.error}<p class="video-error">{video.error}</p>{/if}
      </div>
    {/if}
  </div>
  <div class="transport">
    <button
      class="icon-btn"
      type="button"
      disabled={!player?.path}
      title="Back {JUMP} seconds"
      aria-label="Back {JUMP} seconds"
      onclick={() => video.seek(-JUMP)}><Icon name="rewind" /></button
    >
    <button
      class="icon-btn"
      type="button"
      disabled={!player?.path}
      title="One frame back"
      aria-label="One frame back"
      onclick={() => video.step(true)}><Icon name="stepBack" /></button
    >
    <button
      class="icon-btn"
      type="button"
      disabled={!player?.path}
      aria-label={player?.paused === false ? "Pause" : "Play"}
      onclick={video.togglePlay}><Icon name={player?.paused === false ? "pause" : "play"} /></button
    >
    <button
      class="icon-btn"
      type="button"
      disabled={!player?.path}
      title="One frame on"
      aria-label="One frame on"
      onclick={() => video.step(false)}><Icon name="stepFwd" /></button
    >
    <button
      class="icon-btn"
      type="button"
      disabled={!player?.path}
      title="Forward {JUMP} seconds"
      aria-label="Forward {JUMP} seconds"
      onclick={() => video.seek(JUMP)}><Icon name="forward" /></button
    >
    <span class="timecode">{timecode(player?.time ?? null, player?.fps ?? null)}</span>
    <!-- svelte-ignore a11y_click_events_have_key_events -->
    <div
      class="scrub"
      role="slider"
      tabindex="-1"
      aria-label="Position"
      aria-valuemin={0}
      aria-valuemax={player?.duration ?? 0}
      aria-valuenow={player?.time ?? 0}
      onclick={scrubTo}
    >
      <span style:width="{progress}%"></span>
    </div>
    <button
      class="rate rate-btn"
      type="button"
      disabled={!player?.path}
      title="Playback speed"
      onclick={() => video.speed(1, true)}>{player?.speed ?? 1}×</button
    >
    {#if player?.path}
      <button
        class="rate rate-btn"
        type="button"
        title="The match's first serve is at this frame: every code moves with it"
        onclick={syncHere}>Sync</button
      >
      <button
        class="rate rate-btn"
        type="button"
        title="Open another video"
        onclick={() => openVideo()}>Open…</button
      >
    {/if}
  </div>
</section>
