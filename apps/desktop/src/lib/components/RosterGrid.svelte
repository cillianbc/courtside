<!--
  An editable roster: one row per player, typed like a spreadsheet. Enter
  in the last row adds a player. Used by the team editor and for a match's
  own roster.
-->
<script lang="ts">
  import { tick } from "svelte";
  import {
    ROLES,
    blankPlayer,
    nextNumber,
    rosterProblems,
    suggestCode,
    type PlayerDetail,
  } from "../teams";

  let { rows = $bindable(), details = true }: { rows: PlayerDetail[]; details?: boolean } =
    $props();

  let table: HTMLTableElement | undefined = $state();
  const problems = $derived(rosterProblems(rows));

  async function add() {
    rows.push(blankPlayer(nextNumber(rows)));
    await tick();
    const inputs = table?.querySelectorAll<HTMLInputElement>("tbody tr:last-child input");
    inputs?.[1]?.focus();
  }

  function remove(i: number) {
    rows.splice(i, 1);
  }

  function onkeydown(e: KeyboardEvent, i: number) {
    if (e.key === "Enter" && i === rows.length - 1) {
      e.preventDefault();
      void add();
    }
  }

  // A libero by role is marked libero, and the reverse.
  function setRole(r: PlayerDetail, value: string) {
    r.role = value === "" ? null : Number(value);
    if (r.role === 1) r.libero = true;
    else if (r.libero && r.role !== null) r.libero = false;
  }
</script>

<div class="roster-grid">
  <table bind:this={table}>
    <thead>
      <tr>
        <th class="num">No</th>
        <th>Last name</th>
        <th>First name</th>
        <th>Code</th>
        <th>Role</th>
        <th title="Libero">L</th>
        <th title="Captain">C</th>
        {#if details}
          <th title="Foreign player">Fgn</th>
          <th>Born</th>
          <th title="Height in cm">cm</th>
        {/if}
        <th><span class="sr-only">Remove</span></th>
      </tr>
    </thead>
    <tbody>
      {#each rows as r, i (i)}
        <tr class:bad={problems.has(i)} title={problems.get(i)}>
          <td class="num">
            <input
              type="number"
              min="0"
              max="99"
              bind:value={r.number}
              aria-label="Shirt number"
              onkeydown={(e) => onkeydown(e, i)}
            />
          </td>
          <td>
            <input
              bind:value={r.lastName}
              aria-label="Last name"
              onkeydown={(e) => onkeydown(e, i)}
            />
          </td>
          <td>
            <input
              bind:value={r.firstName}
              aria-label="First name"
              onkeydown={(e) => onkeydown(e, i)}
            />
          </td>
          <td>
            <input
              class="code"
              bind:value={r.code}
              placeholder={suggestCode(r.lastName, r.firstName, r.number)}
              aria-label="Player code (made from the names if left empty)"
              onkeydown={(e) => onkeydown(e, i)}
            />
          </td>
          <td>
            <select
              value={r.role === null ? "" : String(r.role)}
              onchange={(e) => setRole(r, e.currentTarget.value)}
              aria-label="Role"
            >
              <option value="">–</option>
              {#each ROLES as role (role.value)}
                <option value={String(role.value)}>{role.label}</option>
              {/each}
            </select>
          </td>
          <td><input type="checkbox" bind:checked={r.libero} aria-label="Libero" /></td>
          <td><input type="checkbox" bind:checked={r.captain} aria-label="Captain" /></td>
          {#if details}
            <td><input type="checkbox" bind:checked={r.foreign} aria-label="Foreign player" /></td>
            <td>
              <input
                type="date"
                value={r.birthDate ?? ""}
                onchange={(e) => (r.birthDate = e.currentTarget.value || null)}
                aria-label="Date of birth"
              />
            </td>
            <td class="num">
              <input
                type="number"
                min="100"
                max="250"
                value={r.heightCm ?? ""}
                onchange={(e) =>
                  (r.heightCm = e.currentTarget.value ? Number(e.currentTarget.value) : null)}
                aria-label="Height in cm"
              />
            </td>
          {/if}
          <td>
            <button
              type="button"
              class="icon-btn"
              title="Remove from the roster"
              aria-label={`Remove #${r.number}`}
              onclick={() => remove(i)}>×</button
            >
          </td>
        </tr>
      {/each}
    </tbody>
  </table>
  <div class="grid-foot">
    <button class="btn" type="button" onclick={add}>Add player</button>
    <span class="sub-note" class:bad={problems.size > 0}>
      {#if problems.size}
        {[...problems.values()][0]}
      {:else}
        {rows.length} players. Codes left empty are made from the names.
      {/if}
    </span>
  </div>
</div>
