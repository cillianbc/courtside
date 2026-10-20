<!-- The team panel: timeouts, substitutions and rotation first, one row per
     team, then who is on court. Lineups have their own screen. -->
<script lang="ts">
  import * as backend from "../backend";
  import { LAYOUT, columns, type Net } from "../court";
  import { rotationCode, timeoutCode } from "../codes";
  import { withKey } from "../kbd";
  import type { Side, Snapshot } from "../snapshot";
  import { ui } from "../ui.svelte";
  import Icon from "./Icon.svelte";
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

<section class="panel team" aria-label="Team controls and court">
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
  <div class="team-ctl">
    {#each ui.order as side (side)}
      {@const name = snap.teams[side].name}
      <div class="ctl-row" class:away={side === "visiting"} title={name}>
        <SideTag {side} />
        <button
          type="button"
          class="tool"
          title={withKey(`Timeout, ${name}`, side === "home" ? "timeoutHome" : "timeoutAway")}
          disabled={snap.timeouts[side] >= 2}
          onclick={() => ui.commit(timeoutCode(side))}
          ><Icon name="timeout" />Timeout <b>{snap.timeouts[side]}/2</b></button
        >
        <button
          type="button"
          class="tool"
          title={withKey(`Substitution, ${name}`, side === "home" ? "subHome" : "subAway")}
          aria-pressed={ui.subFor === side}
          disabled={snap.needsLineup[side]}
          onclick={() => (ui.subFor = ui.subFor === side ? null : side)}
          ><Icon name="sub" />Sub <b>{snap.substitutions[side]}/{snap.substitutionLimit}</b></button
        >
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
  {#if ui.subFor && !snap.needsLineup[ui.subFor]}
    <SubForm {snap} side={ui.subFor} onclose={() => (ui.subFor = null)} />
  {/if}
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
    <span><span class="sw"></span>Setter</span>
    <span><span class="serve-dot"></span>Server</span>
    <span>Small number: zone</span>
    <span
      >Libero <b class="team-name home" style="font-weight:500">{liberos("home") || "–"}</b> /
      <b class="team-name away" style="font-weight:500">{liberos("visiting") || "–"}</b></span
    >
  </div>
</section>
