<script lang="ts">
  import { substitutionCode } from "../codes";
  import type { Side, Snapshot } from "../snapshot";
  import { ui } from "../ui.svelte";

  let { snap, side, onclose }: { snap: Snapshot; side: Side; onclose: () => void } = $props();

  const team = $derived(snap.teams[side]);
  const court = $derived(snap.court[side]);
  const names = $derived(new Map(team.players.map((p) => [p.number, p.name])));
  const label = (n: number) => {
    const name = names.get(n);
    return name ? `#${n} ${name}` : `#${n}`;
  };
  // Everyone off court who isn't a libero: what an exceptional
  // substitution may bring on.
  const bench = $derived(
    team.players
      .filter((p) => !snap.liberos[side].includes(p.number) && !court.includes(p.number))
      .map((p) => p.number)
      .sort((a, b) => a - b),
  );
  let off = $state("");
  let on = $state("");
  let exceptional = $state(false);
  // The engine's regular-substitution options for the player going off.
  const option = $derived(
    off ? snap.substitutionOptions[side].find((o) => o.off === Number(off)) : undefined,
  );
  const choices = $derived(exceptional ? bench : (option?.on ?? []));
  const code = $derived(off && on ? substitutionCode(side, Number(off), Number(on)) : "");
  const next = $derived(snap.substitutions[side] + 1);

  // A new pick keeps `on` only while it's still offered; a single legal
  // player (a substitute's starter) is picked for the scout.
  function refresh() {
    if (choices.length === 1) on = String(choices[0]);
    else if (!choices.includes(Number(on))) on = "";
  }

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
      <select
        bind:value={off}
        onchange={(e) => {
          off = e.currentTarget.value;
          refresh();
        }}
      >
        <option value="">–</option>
        {#each court as n, i (i)}
          {#if n !== null}<option value={String(n)}>{label(n)} (zone {i + 1})</option>{/if}
        {/each}
      </select>
    </label>
    <label
      >On
      <select bind:value={on} disabled={!off || choices.length === 0}>
        <option value="">–</option>
        {#each choices as n (n)}<option value={String(n)}>{label(n)}</option>{/each}
      </select>
    </label>
    <button class="btn primary" type="submit" disabled={!code}>Make substitution</button>
    <button class="btn" type="button" onclick={onclose}>Cancel</button>
  </div>
  <label class="exceptional"
    ><input
      type="checkbox"
      bind:checked={exceptional}
      onchange={(e) => {
        exceptional = e.currentTarget.checked;
        refresh();
      }}
    /> Exceptional substitution (injury): any bench player</label
  >
  {#if exceptional}
    <div class="sub-note warn" role="status">
      Outside the regular-substitution rule: Checks will flag it if it breaks a pair.
    </div>
  {:else if off && option?.reason}
    <div class="sub-note" role="status">{option.reason}</div>
  {/if}
  <div class="sub-note" class:bad={next > snap.substitutionLimit}>
    {code ? `Writes ${code}. ` : ""}This is substitution {next} of {snap.substitutionLimit}{next >
    snap.substitutionLimit
      ? ", over the season's limit."
      : "."}
  </div>
</form>

<style>
  .exceptional {
    flex-direction: row;
    align-items: center;
    gap: 6px;
    text-transform: none;
    font-size: 12px;
  }
  .exceptional input {
    padding: 0;
    margin: 0;
  }
  .warn {
    color: var(--warn);
  }
</style>
