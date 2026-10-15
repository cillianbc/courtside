<script lang="ts">
  import { onMount } from "svelte";
  import * as backend from "../backend";
  import { roleShort, type PlayerListItem } from "../teams";

  let list = $state<PlayerListItem[]>([]);
  let search = $state("");
  let team = $state("");
  let picked = $state<string[]>([]);
  let error = $state("");

  async function load() {
    list = await backend.players();
  }
  onMount(() => {
    load().catch((e) => (error = String(e)));
  });

  const teams = $derived([...new Map(list.map((p) => [p.teamId, p.teamName])).entries()]);
  const shown = $derived(
    list.filter((p) => {
      const q = search.trim().toLowerCase();
      const text = `${p.player.lastName} ${p.player.firstName} ${p.player.code}`.toLowerCase();
      return (!team || p.teamId === team) && (!q || text.includes(q));
    }),
  );

  function toggle(id: string) {
    picked = picked.includes(id) ? picked.filter((p) => p !== id) : [...picked, id].slice(-2);
  }

  const name = (id: string) => {
    const p = list.find((x) => x.player.id === id)?.player;
    return p ? `${p.lastName} ${p.firstName} (${p.code})` : "";
  };

  async function merge(keep: string, drop: string) {
    error = "";
    try {
      await backend.mergePlayers(keep, drop);
      picked = [];
      await load();
    } catch (e) {
      error = String(e);
    }
  }
</script>

<div class="pick">
  <label class="grow">Search <input bind:value={search} placeholder="name or code" /></label>
  <label
    >Team
    <select bind:value={team}>
      <option value="">All teams</option>
      {#each teams as [id, n] (id)}<option value={id}>{n}</option>{/each}
    </select>
  </label>
</div>
{#if picked.length === 2}
  {@const [a = "", b = ""] = picked}
  <div class="sub-form">
    <div class="title">Merge two records of the same player</div>
    <div class="pick">
      <button class="btn" type="button" onclick={() => merge(a, b)}>Keep {name(a)}</button>
      <button class="btn" type="button" onclick={() => merge(b, a)}>Keep {name(b)}</button>
      <button class="btn" type="button" onclick={() => (picked = [])}>Cancel</button>
    </div>
    <div class="sub-note">The other record's matches move to the one you keep.</div>
  </div>
{:else}
  <p class="sub-note">Tick two players to merge records of the same person.</p>
{/if}
{#if error}<p class="error">{error}</p>{/if}
<table class="list">
  <thead>
    <tr
      ><th></th><th>Player</th><th class="num">No</th><th>Team</th><th>Code</th><th>Role</th><th
        class="num">Matches</th
      ></tr
    >
  </thead>
  <tbody>
    {#each shown as p (p.player.id)}
      <tr>
        <td>
          <input
            type="checkbox"
            checked={picked.includes(p.player.id)}
            onchange={() => toggle(p.player.id)}
            aria-label={`Pick ${p.player.lastName}`}
          />
        </td>
        <td>{p.player.lastName} {p.player.firstName}</td>
        <td class="num">{p.player.number}</td>
        <td>{p.teamName}</td>
        <td class="mono">{p.player.code}</td>
        <td>{roleShort(p.player.role)}{p.player.libero && p.player.role !== 1 ? " L" : ""}</td>
        <td class="num">{p.matches}</td>
      </tr>
    {:else}
      <tr><td colspan="7" class="empty-note">No players.</td></tr>
    {/each}
  </tbody>
</table>
