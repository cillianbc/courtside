<!-- Where plays went (GitHub #11): a court drawn to scale, turned like the
     zones above. The picked code's play is drawn; drag from where it
     started to where it ended, or click where it ended when its code has
     a start zone. -->
<script lang="ts">
  import * as backend from "../backend";
  import { cellPosition, fromDrawing, toDrawing, type Pt } from "../court";
  import { other, type Row, type Side, type Snapshot } from "../snapshot";
  import { keyFor } from "../settings";
  import { ui } from "../ui.svelte";
  import { video } from "../video.svelte";

  let { snap }: { snap: Snapshot } = $props();

  const rows = $derived(snap.groups.flatMap((g) => g.rows));
  const touches = $derived(rows.filter((r) => r.kind === "touch" && r.team !== null));

  // The code being marked: the one picked in the log, else the clip
  // playing, else the last touch coded.
  const target = $derived.by((): Row | null => {
    const picked = touches.find((r) => r.id === ui.selected);
    if (picked) return picked;
    const clip = video.clip && touches.find((r) => r.id === video.clip?.id);
    return clip ?? touches.at(-1) ?? null;
  });

  // Plays of the same rally, drawn faintly behind the picked one.
  const rally = $derived.by((): Row[] => {
    if (!target) return [];
    const g = snap.groups.find((g) => g.rows.some((r) => r.id === target.id));
    return (g?.rows ?? []).filter((r) => r.kind === "touch" && r.id !== target.id);
  });

  // The video layout draws it end on, as a tall strip beside the video.
  const end = $derived(ui.endView || ui.layout === "watch");
  const keys = $derived(
    [keyFor(ui.keymap, "codePrev"), keyFor(ui.keymap, "codeNext")].filter(Boolean).join(" / "),
  );

  // A reception or dig is drawn in the frame of the serve or attack it
  // answers (as .dvw keeps it); every other play in its own team's.
  function frameTeam(r: Row): Side | null {
    if (r.team === null) return null;
    return r.text[3] === "R" || r.text[3] === "D" ? other(r.team) : r.team;
  }

  function mirrored(r: Row): boolean {
    return frameTeam(r) !== (ui.order[0] ?? "home");
  }

  function arrow(r: Row): { from: Pt | null; to: Pt | null } {
    const at = (cell: number | null) => {
      const p = cellPosition(cell);
      return p && toDrawing(p, mirrored(r), end);
    };
    return { from: at(r.start), to: at(r.end) };
  }

  // The end mark: a dot for a winning or perfect play, a cross for an
  // error, an arrowhead when the rally went on.
  function mark(r: Row): "win" | "error" | "on" {
    const ev = r.text[5];
    return ev === "#" ? "win" : ev === "=" ? "error" : "on";
  }

  // The drawing in court units, with room for balls out and serves.
  const box = $derived(end ? "0 -0.35 4 7.7" : "-0.35 0 7.7 4");
  const lines = $derived.by(() => {
    const p = (x: number, y: number) => toDrawing({ x, y }, false, end);
    const seg = (a: Pt, b: Pt) => ({ a, b });
    return {
      court: [p(0.5, 0.5), p(3.5, 6.5)],
      net: seg(p(0.2, 3.5), p(3.8, 3.5)),
      attack: [seg(p(0.5, 2.5), p(3.5, 2.5)), seg(p(0.5, 4.5), p(3.5, 4.5))],
    };
  });
  const rect = $derived.by(() => {
    const [a, b] = lines.court as [Pt, Pt];
    return {
      x: Math.min(a.x, b.x),
      y: Math.min(a.y, b.y),
      w: Math.abs(a.x - b.x),
      h: Math.abs(a.y - b.y),
    };
  });

  let svg: SVGSVGElement | undefined = $state();
  let from = $state<Pt | null>(null);
  let to = $state<Pt | null>(null);

  function local(e: PointerEvent): Pt | null {
    const m = svg?.getScreenCTM();
    if (!m) return null;
    const p = new DOMPoint(e.clientX, e.clientY).matrixTransform(m.inverse());
    return { x: p.x, y: p.y };
  }

  function down(e: PointerEvent) {
    if (!target || e.button !== 0) return;
    from = local(e);
    to = from;
    svg?.setPointerCapture(e.pointerId);
  }

  function move(e: PointerEvent) {
    if (from) to = local(e);
  }

  async function up(e: PointerEvent) {
    const start = from;
    const stop = local(e) ?? to;
    from = null;
    to = null;
    if (!target || !start || !stop) return;
    const flip = mirrored(target);
    const play = (s: Pt): [number, number] => {
      const p = fromDrawing(s, flip, end);
      return [p.x, p.y];
    };
    // Less than a third of a metre is a click: the end only.
    const dragged = Math.hypot(stop.x - start.x, stop.y - start.y) > 0.1;
    const t = target;
    await ui.apply(
      `Marked ${t.text}`,
      () => backend.setPoints(t.id, dragged ? play(start) : null, play(stop)),
      true,
    );
  }

  function clear() {
    const t = target;
    if (t) void ui.apply(`Cleared ${t.text}`, () => backend.setPoints(t.id, null, null), true);
  }
</script>

{#snippet drawn(r: Row, main: boolean)}
  {@const a = arrow(r)}
  {@const cls = r.team === "home" ? "home" : "away"}
  {#if a.from && a.to}
    <line class="play {cls}" class:main x1={a.from.x} y1={a.from.y} x2={a.to.x} y2={a.to.y} />
  {/if}
  {#if a.to}
    {@const m = mark(r)}
    {#if m === "win"}
      <circle class="end {cls}" class:main cx={a.to.x} cy={a.to.y} r="0.09" />
    {:else if m === "error"}
      <path
        class="end-x {cls}"
        class:main
        d="M{a.to.x - 0.08} {a.to.y - 0.08}L{a.to.x + 0.08} {a.to.y + 0.08}M{a.to.x + 0.08} {a.to
          .y - 0.08}L{a.to.x - 0.08} {a.to.y + 0.08}"
      />
    {:else if a.from}
      {@const d = Math.hypot(a.to.x - a.from.x, a.to.y - a.from.y) || 1}
      {@const ux = (a.to.x - a.from.x) / d}
      {@const uy = (a.to.y - a.from.y) / d}
      <path
        class="end {cls}"
        class:main
        d="M{a.to.x} {a.to.y}L{a.to.x - ux * 0.2 - uy * 0.09} {a.to.y - uy * 0.2 + ux * 0.09}L{a.to
          .x -
          ux * 0.2 +
          uy * 0.09} {a.to.y - uy * 0.2 - ux * 0.09}Z"
      />
    {:else}
      <circle class="end {cls}" class:main cx={a.to.x} cy={a.to.y} r="0.07" />
    {/if}
  {/if}
{/snippet}

<section class="panel plays play-court" class:end aria-label="Where plays went">
  <div class="play-head">
    {#if target}
      <span
        >Where it went: <code>{target.text}</code>
        <span class="sub-note">{target.meaning}</span></span
      >
      {#if target.start !== null || target.end !== null}
        <button class="btn" type="button" onclick={clear}>Clear</button>
      {/if}
    {:else}
      <span class="sub-note">Code a play, or pick one in the log, to mark where it went.</span>
    {/if}
  </div>
  <svg
    bind:this={svg}
    viewBox={box}
    role="img"
    aria-label="Court: drag from where the play started to where it ended, or click where it ended"
    onpointerdown={down}
    onpointermove={move}
    onpointerup={up}
    onpointercancel={() => (from = to = null)}
  >
    <rect class="floor" x={rect.x} y={rect.y} width={rect.w} height={rect.h} />
    {#each lines.attack as l, i (i)}
      <line class="court-line thin" x1={l.a.x} y1={l.a.y} x2={l.b.x} y2={l.b.y} />
    {/each}
    <line
      class="net-line"
      x1={lines.net.a.x}
      y1={lines.net.a.y}
      x2={lines.net.b.x}
      y2={lines.net.b.y}
    />
    {#each rally as r (r.id)}
      {@render drawn(r, false)}
    {/each}
    {#if target}{@render drawn(target, true)}{/if}
    {#if from && to}
      <line class="play drag" x1={from.x} y1={from.y} x2={to.x} y2={to.y} />
    {/if}
  </svg>
  <p class="sub-note plays-note">
    Drag from start to end, or click the end when the code has a start zone.
    {#if keys}{keys} pick the previous or next play.{/if}
  </p>
</section>
