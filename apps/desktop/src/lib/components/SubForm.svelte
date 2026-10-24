<script lang="ts">
  import { substitutionCode } from "../codes";
  import type { Side, Snapshot } from "../snapshot";
  import { ui } from "../ui.svelte";

  let { snap, side, onclose }: { snap: Snapshot; side: Side; onclose: () => void } = $props();

  const team = $derived(snap.teams[side]);
  const court = $derived(snap.court[side]);
  const bench = $derived(
    team.players
      .filter((p) => !p.libero && !court.includes(p.number))
      .map((p) => p.number)
      .sort((a, b) => a - b),
  );
  let off = $state("");
  let on = $state("");
  const code = $derived(off && on ? substitutionCode(side, Number(off), Number(on)) : "");
  const next = $derived(snap.substitutions[side] + 1);

  async function submit(e: SubmitEvent) {
    e.preventDefault();
    if (!code) return;
    if (await ui.commit(code)) onclose();
  }
</script>

<svelte:window
  onkeydown={(e) => {
    if (e.key === "Escape") onclose();
  }}
/>

<form class="sub-form" onsubmit={submit}>
  <div class="title">Substitution, {team.name}</div>
  <div class="pick">
    <label
      >Off
      <select bind:value={off}>
        <option value="">–</option>
        {#each court as n, i (i)}
          {#if n !== null}<option value={String(n)}>#{n} (zone {i + 1})</option>{/if}
        {/each}
      </select>
    </label>
    <label
      >On
      <select bind:value={on}>
        <option value="">–</option>
        {#each bench as n (n)}<option value={String(n)}>#{n}</option>{/each}
      </select>
    </label>
    <button class="btn primary" type="submit" disabled={!code}>Make substitution</button>
    <button class="btn" type="button" onclick={onclose}>Cancel</button>
  </div>
  <div class="sub-note" class:bad={next > snap.substitutionLimit}>
    {code ? `Writes ${code}. ` : ""}This is substitution {next} of {snap.substitutionLimit}{next >
    snap.substitutionLimit
      ? ", over the season's limit."
      : "."}
  </div>
</form>
