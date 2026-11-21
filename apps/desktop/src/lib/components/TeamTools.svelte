<!-- One team's timeout and substitution buttons. The Coding layout puts them
     in the team panel's rows; the Video layout in the Rotation panel's
     header. The substitution button opens the popover in Court.svelte, which
     finds it by `data-sub-toggle`. -->
<script lang="ts">
  import { timeoutCode } from "../codes";
  import { withKey } from "../kbd";
  import type { Side, Snapshot } from "../snapshot";
  import { ui } from "../ui.svelte";
  import Icon from "./Icon.svelte";
  import Kbd from "./Kbd.svelte";

  let { snap, side, keys = true }: { snap: Snapshot; side: Side; keys?: boolean } = $props();

  const name = $derived(snap.teams[side].name);
  const toOut = $derived(snap.timeouts[side] >= 2);
  const subOut = $derived(snap.substitutions[side] >= snap.substitutionLimit);
  const open = $derived(ui.subFor === side && !snap.needsLineup[side]);
  const toKey = $derived(side === "home" ? "timeoutHome" : "timeoutAway");
  const subKey = $derived(side === "home" ? "subHome" : "subAway");

  function toggleSub() {
    if (ui.subFor === side) {
      ui.subFor = null;
      ui.focusCode += 1;
    } else {
      ui.subFor = side;
    }
  }
</script>

<button
  type="button"
  class="tool"
  title={toOut ? `No timeouts left in this set, ${name}` : withKey(`Timeout, ${name}`, toKey)}
  aria-label="Timeout, {name}"
  disabled={toOut}
  onclick={() => ui.commit(timeoutCode(side))}
  ><Icon name="timeout" /><span class="num">{snap.timeouts[side]}/2</span>{#if keys}<Kbd
      action={toKey}
    />{/if}</button
>
<button
  type="button"
  class="tool"
  title={subOut
    ? `${snap.substitutionLimit} of ${snap.substitutionLimit} substitutions used, ${name}`
    : withKey(`Substitution, ${name}`, subKey)}
  aria-label="Substitution, {name}"
  aria-haspopup="dialog"
  aria-expanded={open}
  aria-controls={open ? "sub-pop" : undefined}
  data-sub-toggle={side}
  disabled={snap.needsLineup[side] || subOut}
  onclick={toggleSub}
  ><Icon name="sub" /><span class="num">{snap.substitutions[side]}/{snap.substitutionLimit}</span
  >{#if keys}<Kbd action={subKey} />{/if}</button
>
