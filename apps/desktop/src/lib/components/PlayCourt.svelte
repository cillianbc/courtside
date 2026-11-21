<!-- Where plays went (GitHub #11): a court drawn to scale, turned like the
     zones above. The picked code's play is drawn; drag from where it
     started to where it ended, or click where it ended when its code has
     a start zone (a set or block: where it was). The mark fills the
     code's zones. -->
<script lang="ts">
  import * as backend from "../backend";
  import { rotationCode } from "../codes";
  import { cellPosition, fromDrawing, toDrawing, zonePosition, type Pt } from "../court";
  import { withKey } from "../kbd";
  import { rotationName, rotationOf, type RotationView } from "../rotation";
  import Icon from "./Icon.svelte";
  import { other, type Row, type Side, type Snapshot } from "../snapshot";
  import { keyFor } from "../settings";
  import { ui } from "../ui.svelte";
  import { video } from "../video.svelte";

  let { snap }: { snap: Snapshot } = $props();

  const rows = $derived(snap.groups.flatMap((g) => g.rows));
  const touches = $derived(rows.filter((r) => r.kind === "touch" && r.team !== null));

  // The code being marked: the one picked in the log, else the clip
  // playing, else the last serve, attack or free ball of the rally being
  // coded (the touch after it is usually a dig or cover), else its last
  // touch.
  const target = $derived.by((): Row | null => {
    const picked = touches.find((r) => r.id === ui.selected);
    if (picked) return picked;
    const clip = video.clip && touches.find((r) => r.id === video.clip?.id);
    if (clip) return clip;
    const last = [...snap.groups].reverse().find((g) => g.rows.some((r) => r.kind === "touch"));
    const plays = (last?.rows ?? []).filter((r) => r.kind === "touch" && r.team !== null);
    const play = [...plays].reverse().find((r) => "SAF".includes(r.text[3] ?? ""));
    return play ?? plays.at(-1) ?? null;
  });

  // A mark writes the code's zones, so the code comes back as a new
  // version: keep it picked.
  async function marking(t: Row, change: () => Promise<Snapshot>) {
    const s = await change();
    if (ui.selected === t.id) {
      ui.selected = s.groups.flatMap((g) => g.rows).find((r) => r.index === t.index)?.id ?? null;
    }
    return s;
  }

  // Plays of the same rally, drawn faintly behind the picked one.
  const rally = $derived.by((): Row[] => {
    if (!target) return [];
    const g = snap.groups.find((g) => g.rows.some((r) => r.id === target.id));
    return (g?.rows ?? []).filter((r) => r.kind === "touch" && r.id !== target.id);
  });

  // The video layout draws it end on, as a tall strip beside the video.
  const end = $derived(ui.endView || ui.layout === "watch");
  const watch = $derived(ui.layout === "watch");

  // The Video layout's one court (as VolleyStation): who is on court, drawn
  // as shirts by zone, at the picked code (the backend's fold there) or at
  // the end of the log. Alt+P hides them.
  let picked = $state<RotationView | null>(null);
  $effect(() => {
    const id = ui.selected;
    void snap; // a change to the log can change who was on court there
    if (!id || !watch) {
      picked = null;
      return;
    }
    let live = true;
    backend.rotationAt(id).then(
      (r) => {
        if (live) picked = r;
      },
      () => {
        if (live) picked = null;
      },
    );
    return () => {
      live = false;
    };
  });
  const view = $derived(picked ?? rotationOf(snap));
  interface Player {
    side: Side;
    zone: number;
    number: number;
    at: Pt;
    setter: boolean;
    server: boolean;
  }
  const players = $derived.by((): Player[] => {
    if (!watch || !ui.rotationShown) return [];
    const out: Player[] = [];
    for (const side of ui.order) {
      const flip = side !== (ui.order[0] ?? "home");
      view.court[side].forEach((n, i) => {
        if (n === null || n === undefined) return;
        const zone = i + 1;
        out.push({
          side,
          zone,
          number: n,
          at: toDrawing(zonePosition(zone), flip, end),
          setter: n === view.setter[side],
          server: view.serving === side && zone === 1,
        });
      });
    }
    return out;
  });
  // A shirt, centred on (0, 0), in court units.
  const SHIRT =
    "M-0.17 -0.3L-0.36 -0.2L-0.3 0.0L-0.2 -0.04L-0.2 0.3L0.2 0.3L0.2 -0.04L0.3 0.0L0.36 -0.2L0.17 -0.3Q0 -0.2 -0.17 -0.3Z";
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

  // On a short window the Coding layout folds the court to its header until
  // a play is picked or a clip plays, or the scout opens it (v6 review). The
  // Video layout's court is also the rotation, so it stays open.
  let opened = $state(false);
  const marking_ = $derived(watch || opened || !!ui.selected || !!video.clip);

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
      () => marking(t, () => backend.setPoints(t.id, dragged ? play(start) : null, play(stop))),
      true,
    );
  }

  function clear() {
    const t = target;
    if (t)
      void ui.apply(
        `Cleared ${t.text}`,
        () => marking(t, () => backend.setPoints(t.id, null, null)),
        true,
      );
  }
</script>

{#snippet rotate(side: Side)}
  {@const name = snap.teams[side].name}
  <div class="court-rot">
    <span class="court-p" title="{name}: setter in zone {view.setterZone[side] ?? '?'}"
      >{rotationName(view.setterZone[side])}</span
    >
    {#if view.liberos[side].length}<span class="court-l" title="Libero"
        >L {view.liberos[side].join(", ")}</span
      >{/if}
    {#each [true, false] as forward (forward)}
      {@const code = rotationCode(side, snap.setterZone[side], forward)}
      {@const label = `Rotate ${name} ${forward ? "forward" : "back"} one place`}
      <button
        type="button"
        class="icon-btn"
        title={forward ? withKey(label, side === "home" ? "rotateHome" : "rotateAway") : label}
        aria-label={label}
        disabled={code === null}
        onclick={() => code && ui.commit(code)}><Icon name={forward ? "fwd" : "back"} /></button
      >
    {/each}
  </div>
{/snippet}

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

<section
  class="panel plays play-court"
  class:end
  class:marking={marking_}
  aria-label="Where plays went"
>
  <div class="play-head">
    {#if target}
      <span
        >Where it went: <code>{target.text}</code>
        <span class="sub-note">{target.meaning}</span></span
      >
      {#if picked}<span class="sub-note court-at"
          >Set {view.set} · {view.score.home}–{view.score.visiting}</span
        >{/if}
      <button
        class="btn mark-open"
        type="button"
        aria-pressed={opened}
        title="Show the court to mark this play"
        onclick={() => (opened = !opened)}>Mark</button
      >
      {#if target.start !== null || target.end !== null}
        <button class="btn" type="button" onclick={clear}>Clear</button>
      {/if}
    {:else}
      <span class="sub-note">Code a play, or pick one in the log, to mark where it went.</span>
    {/if}
  </div>
  <div class="court-wrap" class:with-rot={watch}>
    {#if watch}{@render rotate(ui.order[0] ?? "home")}{/if}
    <svg
      class="court-svg"
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
      {#each players as p (p.side + p.zone)}
        <g
          class="shirt {p.side === 'home' ? 'home' : 'away'}"
          class:setter={p.setter}
          transform="translate({p.at.x} {p.at.y}) scale(1.4)"
        >
          <title
            >{snap.teams[p.side].name}
            {p.number}, zone {p.zone}{p.setter ? ", setter" : ""}{p.server
              ? ", serving"
              : ""}</title
          >
          <path d={SHIRT} />
          <text y="0.1">{p.number}</text>
          {#if p.setter}<circle class="s-badge" cx="0.25" cy="0.25" r="0.14" /><text
              class="s-text"
              x="0.25"
              y="0.32">S</text
            >{/if}
          {#if p.server}<circle class="serve-ball" cx="-0.26" cy="0.26" r="0.08" />{/if}
        </g>
      {/each}
      {#each rally as r (r.id)}
        {@render drawn(r, false)}
      {/each}
      {#if target}{@render drawn(target, true)}{/if}
      {#if from && to}
        <line class="play drag" x1={from.x} y1={from.y} x2={to.x} y2={to.y} />
      {/if}
    </svg>
    {#if watch}{@render rotate(ui.order[1] ?? "visiting")}{/if}
  </div>
  <div class="end-legend" aria-label="End marks">
    <span
      ><svg viewBox="0 0 18 12" aria-hidden="true"
        ><path d="M2 6h10" stroke="currentColor" stroke-width="1.6" /><circle
          cx="13"
          cy="6"
          r="3.4"
          fill="currentColor"
        /></svg
      >point</span
    >
    <span
      ><svg viewBox="0 0 18 12" aria-hidden="true"
        ><path d="M2 6h9" stroke="currentColor" stroke-width="1.6" /><path
          d="M10.5 2.5l6 7M16.5 2.5l-6 7"
          stroke="currentColor"
          stroke-width="1.8"
        /></svg
      >error</span
    >
    <span
      ><svg viewBox="0 0 18 12" aria-hidden="true"
        ><path d="M2 6h10" stroke="currentColor" stroke-width="1.6" /><path
          d="M11 2l6 4-6 4Z"
          fill="currentColor"
        /></svg
      >rally went on</span
    >
  </div>
  <p class="sub-note plays-note">
    Drag from start to end, or click the end when the code has a start zone; a set or block is
    clicked where it was. The mark writes the code's zones; Clear puts back the typed ones.
    {#if keys}{keys} pick the previous or next play.{/if}
  </p>
</section>
