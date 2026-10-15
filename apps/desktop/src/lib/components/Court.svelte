<script lang="ts">
  import { LAYOUT, columns, type Net } from "../court";
  import { rotationCode, timeoutCode } from "../codes";
  import type { Side, Snapshot } from "../snapshot";
  import { ui } from "../ui.svelte";
  import Icon from "./Icon.svelte";
  import LineupForm from "./LineupForm.svelte";
  import SideTag from "./SideTag.svelte";
  import SubForm from "./SubForm.svelte";

  let { snap }: { snap: Snapshot } = $props();

  const nets = $derived<[Net, Net]>(ui.endView ? ["bottom", "top"] : ["right", "left"]);

  function liberos(side: Side): string {
    return snap.teams[side].players
      .filter((p) => p.libero)
      .map((p) => p.number)
      .join(", ");
  }
</script>

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
        </div>
      {/each}
    </div>
  </div>
{/snippet}

<section class="panel" aria-label="On court">
  <div class="court-head">
    <h2>On court · set {snap.set}</h2>
    <div class="view-tools">
      <button
        class="icon-btn"
        type="button"
        title="Swap sides"
        aria-label="Swap sides"
        onclick={() => (ui.swapped = !ui.swapped)}><Icon name="swap" /></button
      >
      <button
        class="icon-btn"
        type="button"
        title={ui.endView ? "Side view" : "End view"}
        aria-label={ui.endView ? "Side view" : "End view"}
        aria-pressed={ui.endView}
        onclick={() => (ui.endView = !ui.endView)}><Icon name="turn" /></button
      >
    </div>
  </div>
  {#if snap.matchOver}
    <p class="empty-note">Match over. Corrections still work; export the .dvw from the top bar.</p>
  {/if}
  <div class="courts" class:end={ui.endView}>
    {@render half(ui.order[0] ?? "home", nets[0])}
    <div class="net" aria-hidden="true"></div>
    {@render half(ui.order[1] ?? "visiting", nets[1])}
  </div>
  <div class="court-legend">
    <span><span class="sw"></span>Setter</span>
    <span><span class="serve-dot"></span>Server</span>
    <span>Small number: zone</span>
    <span
      >Libero <b class="team-name home" style="font-weight:500">{liberos("home") || "–"}</b> /
      <b class="team-name away" style="font-weight:500">{liberos("visiting") || "–"}</b></span
    >
  </div>
  <div class="team-ctl">
    {#each ui.order as side (side)}
      {@const name = snap.teams[side].name}
      <div class="ctl-row" class:away={side === "visiting"}>
        <div class="name">
          <span class="team-name"><SideTag {side} />{name}</span>
          <span>setter zone {snap.setterZone[side] ?? "?"}</span>
        </div>
        <button
          type="button"
          class="tool"
          disabled={snap.timeouts[side] >= 2}
          onclick={() => ui.commit(timeoutCode(side))}
          ><Icon name="timeout" />Timeout <b>{snap.timeouts[side]}/2</b></button
        >
        <button
          type="button"
          class="tool"
          title="Correct the six this set started with"
          aria-pressed={ui.lineupFor === side}
          disabled={!snap.starting[side] || snap.matchOver}
          onclick={() => {
            ui.lineupFor = ui.lineupFor === side ? null : side;
            ui.subFor = null;
          }}><Icon name="lineup" />Lineup</button
        >
        <button
          type="button"
          class="tool"
          aria-pressed={ui.subFor === side}
          disabled={snap.needsLineup[side]}
          onclick={() => {
            ui.subFor = ui.subFor === side ? null : side;
            ui.lineupFor = null;
          }}><Icon name="sub" />Sub <b>{snap.substitutions[side]}/6</b></button
        >
        <span class="rot-group">
          <span>Rotation</span>
          {#each [false, true] as forward (forward)}
            {@const code = rotationCode(side, snap.setterZone[side], forward)}
            {@const label = `Rotate ${name} ${forward ? "forward" : "back"} one place`}
            <button
              type="button"
              class="icon-btn"
              title={label}
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
  {#each ui.order as side (side)}
    {#if snap.needsLineup[side]}<LineupForm {snap} {side} />{/if}
  {/each}
  {#if ui.lineupFor && snap.starting[ui.lineupFor]}
    {#key ui.lineupFor}
      <LineupForm {snap} side={ui.lineupFor} onclose={() => (ui.lineupFor = null)} />
    {/key}
  {/if}
  {#if ui.subFor && !snap.needsLineup[ui.subFor]}
    <SubForm {snap} side={ui.subFor} onclose={() => (ui.subFor = null)} />
  {/if}
</section>
