<!-- The team panel: timeouts, substitutions and rotation first, one row per
     team, then who is on court. Lineups have their own screen. The
     substitution form opens as a popover under its team's row (#76), over the
     panel and what lies below it, so the column keeps its layout. -->
<script lang="ts">
  import * as backend from "../backend";
  import { LAYOUT, columns, type Net } from "../court";
  import { rotationCode } from "../codes";
  import { withKey } from "../kbd";
  import type { Side, Snapshot } from "../snapshot";
  import { ui } from "../ui.svelte";
  import Icon from "./Icon.svelte";
  import SetterBadge from "./SetterBadge.svelte";
  import SideTag from "./SideTag.svelte";
  import SubForm from "./SubForm.svelte";
  import TeamTools from "./TeamTools.svelte";

  let { snap }: { snap: Snapshot } = $props();

  const nets = $derived<[Net, Net]>(ui.endView ? ["bottom", "top"] : ["right", "left"]);

  function liberos(side: Side): string {
    return snap.liberos[side].join(", ");
  }

  // The substitution popover (#76): fixed-positioned outside the panel (the
  // panel scrolls and is a size container, which would clip it), placed under
  // the row whose button opened it, or above it when there's no room below.
  // The button is in this panel's rows (Coding layout) or the Rotation
  // panel's header (Video layout); either way it carries `data-sub-toggle`.
  let section = $state<HTMLElement>();
  let pop = $state<HTMLElement>();
  const subSide = $derived(ui.subFor && !snap.needsLineup[ui.subFor] ? ui.subFor : null);
  let place = $state({ top: 0, left: 0, width: 0, tip: 0, above: false });

  function position() {
    const toggle = subSide
      ? document.querySelector<HTMLElement>(`[data-sub-toggle="${subSide}"]`)
      : null;
    const row = toggle?.closest<HTMLElement>(".ctl-row, .tools-line") ?? null;
    const host = toggle?.closest<HTMLElement>(".panel") ?? section;
    if (!toggle || !row || !pop || !host) return;
    const panel = host.getBoundingClientRect();
    const r = row.getBoundingClientRect();
    const btn = toggle.getBoundingClientRect();
    const h = pop.offsetHeight;
    const above = r.bottom + 8 + h > window.innerHeight - 8 && r.top - 8 - h >= 8;
    const width = Math.min(Math.max(panel.width - 8, 260), window.innerWidth - 16);
    const left = Math.max(8, Math.min(panel.left + 4, window.innerWidth - width - 8));
    place = {
      top: above ? r.top - 8 - h : r.bottom + 8,
      left,
      width,
      tip: btn.left + btn.width / 2 - left - 6,
      above,
    };
  }

  $effect(() => {
    if (!subSide || !pop) return;
    position();
    const ro = new ResizeObserver(position);
    ro.observe(pop);
    if (section) ro.observe(section);
    window.addEventListener("resize", position);
    // The panel scrolls in the Coding layout; the popover follows its row.
    window.addEventListener("scroll", position, true);
    return () => {
      ro.disconnect();
      window.removeEventListener("resize", position);
      window.removeEventListener("scroll", position, true);
    };
  });

  // Focus the form's first field when it opens for a team.
  $effect(() => {
    if (!subSide || !pop) return;
    const el = pop;
    queueMicrotask(() =>
      el.querySelector<HTMLElement>("select, input, button:not(:disabled)")?.focus(),
    );
  });

  /** Closes the popover; `refocus` puts the caret back in the command line. */
  function closeSub(refocus = true) {
    ui.subFor = null;
    if (refocus) ui.focusCode += 1;
  }

  // A press outside the popover and its buttons closes it, leaving focus be.
  function outside(e: PointerEvent) {
    if (!subSide || !(e.target instanceof Node)) return;
    if (pop?.contains(e.target)) return;
    if (e.target instanceof Element && e.target.closest("[data-sub-toggle]")) return;
    closeSub(false);
  }
</script>

<svelte:window onpointerdowncapture={outside} />

{#snippet half(side: Side, net: Net)}
  {@const cls = side === "home" ? "home" : "away"}
  <div class="half-wrap">
    <div class="half-label {cls}"><SideTag {side} />{snap.teams[side].name}</div>
    <div class="half {cls}" class:wide={columns(net) === 3}>
      {#each LAYOUT[net] as zone (zone)}
        {@const n = snap.court[side][zone - 1] ?? null}
        <div
          class="zone"
          class:setter={n !== null && n === snap.setter[side]}
          class:server={side === snap.serving && zone === 1}
        >
          <small>{zone}</small>{n ?? ""}
          {#if n !== null && n === snap.setter[side]}<SetterBadge />{/if}
        </div>
      {/each}
    </div>
  </div>
{/snippet}

<section class="panel team" aria-label="Team controls and court" bind:this={section}>
  {#if snap.matchOver}
    <div class="match-over">
      <p class="empty-note">
        Match over. Codes can still be modified; export the .dvw from the top bar.
      </p>
      {#if snap.goldenSet === null}
        <button
          class="btn"
          type="button"
          title="A deciding set to 15 after a two-legged tie (CEV)"
          onclick={() => ui.apply("Golden set started", backend.startGoldenSet)}
          >Play golden set</button
        >
      {/if}
    </div>
  {:else if snap.goldenSet === snap.set}
    <p class="empty-note">Golden set: first to 15, two clear.</p>
  {/if}
  {#if ui.layout === "watch"}
    <!-- The Video layout: every timeout and substitution on one line, the
         teams mirrored left and right in court order (#76). -->
    <div class="tools-line" role="group" aria-label="Timeouts and substitutions">
      {#each ui.order as side, i (side)}
        <span
          class="tools-team {side === 'home' ? 'home' : 'away'}"
          class:right={i === 1}
          role="group"
          aria-label={snap.teams[side].name}
          ><TeamTools {snap} {side} keys={false} subsFirst={i === 0} /></span
        >
      {/each}
    </div>
  {/if}
  <div class="team-ctl">
    {#each ui.order as side (side)}
      {@const name = snap.teams[side].name}
      <div class="ctl-row" class:away={side === "visiting"}>
        <span class="who">
          <b class="team-name {side === 'home' ? 'home' : 'away'}" title={name}>{name}</b>
          <span class="setter-in" title="Setter in zone {snap.setterZone[side] ?? '?'}"
            ><SetterBadge /><span class="lbl">setter in</span> {snap.setterZone[side] ?? "?"}</span
          >
        </span>
        {#if ui.layout !== "watch"}<TeamTools {snap} {side} />{/if}
        <span class="rot-group">
          {#each [false, true] as forward (forward)}
            {@const code = rotationCode(side, snap.setterZone[side], forward)}
            {@const label = `Rotate ${name} ${forward ? "forward" : "back"} one place (setter in zone ${snap.setterZone[side] ?? "?"})`}
            <button
              type="button"
              class="icon-btn"
              title={forward
                ? withKey(label, side === "home" ? "rotateHome" : "rotateAway")
                : label}
              aria-label={label}
              disabled={code === null}
              onclick={() => code && ui.commit(code)}
              ><Icon name={forward ? "fwd" : "back"} /></button
            >
          {/each}
        </span>
      </div>
    {/each}
  </div>
  <div class="court-head">
    <h2>On court · set {snap.set}</h2>
    <div class="view-tools">
      <button
        class="icon-btn"
        type="button"
        title={withKey("Swap sides", "swapSides")}
        aria-label="Swap sides"
        onclick={() => (ui.swapped = !ui.swapped)}><Icon name="swap" /></button
      >
      <button
        class="icon-btn"
        type="button"
        title={withKey(ui.endView ? "Side view" : "End view", "endView")}
        aria-label={ui.endView ? "Side view" : "End view"}
        aria-pressed={ui.endView}
        onclick={() => (ui.endView = !ui.endView)}><Icon name="turn" /></button
      >
    </div>
  </div>
  <div class="courts" class:end={ui.endView}>
    {@render half(ui.order[0] ?? "home", nets[0])}
    <div class="net" aria-hidden="true"></div>
    {@render half(ui.order[1] ?? "visiting", nets[1])}
  </div>
  <div class="court-legend">
    <span><SetterBadge />Setter (ring and S)</span>
    <span><span class="serve-dot"></span>Server</span>
    <span>Small number: zone</span>
    <span
      >Libero <b class="team-name home" style="font-weight:500">{liberos("home") || "–"}</b> /
      <b class="team-name away" style="font-weight:500">{liberos("visiting") || "–"}</b></span
    >
  </div>
</section>
{#if subSide}
  <div
    class="sub-pop"
    class:above={place.above}
    id="sub-pop"
    role="dialog"
    aria-label="Substitution, {snap.teams[subSide].name}"
    tabindex="-1"
    bind:this={pop}
    style:top="{place.top}px"
    style:left="{place.left}px"
    style:width="{place.width}px"
    style:--tip="{place.tip}px"
    onkeydown={(e) => {
      if (e.key !== "Escape") return;
      e.preventDefault();
      e.stopPropagation();
      closeSub();
    }}
  >
    {#key subSide}<SubForm {snap} side={subSide} onclose={() => closeSub()} />{/key}
  </div>
{/if}
