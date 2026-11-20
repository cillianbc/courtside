<!-- The Video layout's rotation (#74): both teams' six by zone, the server,
     the setter's rotation and the liberos, at the picked code or the end
     of the log. Beside the video, never over it: the picture is a native
     layer HTML can't cover. -->
<script lang="ts">
  import * as backend from "../backend";
  import { columns, type Net } from "../court";
  import { withKey } from "../kbd";
  import { cells, rotationName, rotationOf, type RotationView } from "../rotation";
  import type { Side, Snapshot } from "../snapshot";
  import { ui } from "../ui.svelte";

  let { snap }: { snap: Snapshot } = $props();

  // The fold at the picked code; null follows the end of the log.
  let picked = $state<RotationView | null>(null);
  $effect(() => {
    const id = ui.selected;
    void snap; // a change to the log can change who was on court there
    if (!id || !ui.rotationShown) {
      picked = null;
      return;
    }
    let live = true;
    backend.rotationAt(id).then(
      (r) => {
        if (live) picked = r;
      },
      () => {
        // A code taken out since it was picked: the end of the log.
        if (live) picked = null;
      },
    );
    return () => {
      live = false;
    };
  });
  const view = $derived(picked ?? rotationOf(snap));
  const nets = $derived<[Net, Net]>(ui.endView ? ["bottom", "top"] : ["right", "left"]);
</script>

{#snippet half(side: Side, net: Net)}
  {@const cls = side === "home" ? "home" : "away"}
  {@const name = snap.teams[side].name}
  {@const libs = view.liberos[side]}
  <div class="rot-half {cls}">
    <div class="rot-label">
      {#if view.serving === side}<span class="serve-dot" title="Serving"></span>{/if}
      <b class="rot-name" title={name}>{name}</b>
      <span
        class="rot-p"
        title="Setter in zone {view.setterZone[side] ?? '?'} (rotation {rotationName(
          view.setterZone[side],
        )})">{rotationName(view.setterZone[side])}</span
      >
      <span class="rot-lib" title="Libero{libs.length === 1 ? '' : 's'}"
        >L {libs.length ? libs.join(", ") : "–"}</span
      >
    </div>
    <div class="rot-grid" class:wide={columns(net) === 3}>
      {#each cells(view, side, net) as c (c.zone)}
        <div
          class="rot-zone"
          class:setter={c.setter}
          class:server={c.server}
          title="Zone {c.zone}{c.number !== null ? `: ${c.number}` : ''}{c.setter
            ? ', setter'
            : ''}{c.server ? ', serving' : ''}"
        >
          <small>{c.zone}</small>{c.number ?? "–"}
        </div>
      {/each}
    </div>
  </div>
{/snippet}

<section class="panel rotation" aria-label="Rotation">
  <div class="rot-head">
    <h2>Rotation</h2>
    {#if ui.rotationShown}
      <span class="rot-at">
        {#if picked}
          Picked code · set {view.set} · {view.score.home}–{view.score.visiting}
        {:else}
          Now
        {/if}
      </span>
    {/if}
    <button
      class="btn small"
      type="button"
      title={withKey(ui.rotationShown ? "Hide the rotation" : "Show the rotation", "rotationView")}
      aria-expanded={ui.rotationShown}
      onclick={() => ui.toggleRotation()}>{ui.rotationShown ? "Hide" : "Show"}</button
    >
  </div>
  {#if ui.rotationShown}
    <div class="rot-courts" class:end={ui.endView}>
      {@render half(ui.order[0] ?? "home", nets[0])}
      <div class="net" aria-hidden="true"></div>
      {@render half(ui.order[1] ?? "visiting", nets[1])}
    </div>
  {/if}
</section>
