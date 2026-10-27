<!--
  A team as DataVolley and VolleyStation hold it (issue #49): details,
  staff, home venue, kit colours, the screen colour and the roster. Used on
  the Teams tab and, from a match, in TeamDialog.
-->
<script lang="ts">
  import * as backend from "../backend";
  import {
    STAFF_ROLES,
    TEAM_COLOURS,
    rosterProblems,
    teamForm,
    type PlayerDetail,
    type TeamDetail,
  } from "../teams";
  import RosterGrid from "./RosterGrid.svelte";

  let { team, onclose }: { team: TeamDetail | null; onclose: (saved: TeamDetail | null) => void } =
    $props();

  // Edits happen on plain copies (snapshots: a reactive proxy can't be
  // cloned); nothing is saved until Save.
  // svelte-ignore state_referenced_locally
  let form = $state(teamForm($state.snapshot(team)));
  // svelte-ignore state_referenced_locally
  let rows = $state<PlayerDetail[]>($state.snapshot(team?.players ?? []));
  let error = $state("");
  let saving = $state(false);

  const KITS = [
    { key: "shirtColour", label: "Shirt", note: "Written to the .dvw for DataVolley." },
    { key: "liberoColour", label: "Libero shirt", note: "" },
  ] as const;

  async function save(e: SubmitEvent) {
    e.preventDefault();
    error = "";
    if (!form.name.trim()) {
      error = "Enter the team's name.";
      return;
    }
    const problems = rosterProblems(rows);
    if (problems.size) {
      error = [...problems.values()][0] ?? "";
      return;
    }
    saving = true;
    try {
      const saved = await backend.saveTeam(
        team?.id ?? null,
        $state.snapshot(form),
        $state.snapshot(rows),
      );
      onclose(saved);
    } catch (err) {
      error = String(err);
    } finally {
      saving = false;
    }
  }
</script>

<form class="panel team-editor" onsubmit={save}>
  <h2>{team ? `Edit ${team.name}` : "New team"}</h2>
  <div class="pick">
    <label class="grow">Name <input bind:value={form.name} placeholder="Harbour City VC" /></label>
    <label
      >Code
      <input
        class="short"
        bind:value={form.code}
        placeholder="auto"
        title="The team's code in the season and in .dvw files. Left empty: made from the name"
      />
    </label>
    <label
      >Short code
      <input
        class="short"
        maxlength="5"
        bind:value={form.shortCode}
        placeholder={form.code.slice(0, 5)}
        title="Up to five characters, for .sq and .vst team files"
      />
    </label>
  </div>

  <fieldset class="team-section">
    <legend>Staff</legend>
    <div class="staff-grid">
      <span></span><span class="col-head">Last name</span><span class="col-head">First name</span>
      {#each STAFF_ROLES as role (role.key)}
        <span class="role" title={role.note}>{role.label}</span>
        <input bind:value={form.staff[role.key].lastName} aria-label={`${role.label}: last name`} />
        <input
          bind:value={form.staff[role.key].firstName}
          aria-label={`${role.label}: first name`}
        />
      {/each}
    </div>
    <p class="sub-note">
      .dvw and .sq files hold the head coach and assistant; .vst files hold all five.
    </p>
  </fieldset>

  <fieldset class="team-section">
    <legend>Home venue</legend>
    <div class="pick">
      <label class="grow">Hall <input bind:value={form.hall} /></label>
      <label class="grow">City <input bind:value={form.city} /></label>
    </div>
    <p class="sub-note">A new match at home starts with this venue; you can change it per match.</p>
  </fieldset>

  <fieldset class="team-section">
    <legend>Kit</legend>
    <div class="kits">
      {#each KITS as kit (kit.key)}
        <div class="kit">
          <span class="role">{kit.label}</span>
          <input
            type="color"
            value={form[kit.key] || "#ffffff"}
            oninput={(e) => (form[kit.key] = e.currentTarget.value)}
            aria-label={`${kit.label} colour`}
          />
          {#if form[kit.key]}
            <span class="mono">{form[kit.key]}</span>
            <button class="btn" type="button" onclick={() => (form[kit.key] = "")}>Clear</button>
          {:else}
            <span class="sub-note">Not set</span>
          {/if}
          {#if kit.note}<span class="sub-note">{kit.note}</span>{/if}
        </div>
      {/each}
    </div>
  </fieldset>

  <fieldset class="colours">
    <legend>Screen colour</legend>
    <p class="sub-note">
      How Courtside shows the team, whatever its kit. Light and dark tones, each easy to read on the
      panels. Colours that look like an error, a warning, a selection or the setter aren't offered.
      When both teams in a match pick the same one, the visiting team keeps its default.
    </p>
    <div class="swatches">
      <label class="sw" class:on={form.colour === ""}>
        <input type="radio" name="colour" value="" bind:group={form.colour} />
        <span class="chip default"><i></i><i></i></span>Default
      </label>
      {#each TEAM_COLOURS as c (c.key)}
        <label class="sw" class:on={form.colour === c.key}>
          <input type="radio" name="colour" value={c.key} bind:group={form.colour} />
          <span class="chip"
            ><i style:background={c.light}></i><i style:background={c.dark}></i></span
          >{c.label}
        </label>
      {/each}
    </div>
  </fieldset>

  <RosterGrid bind:rows />
  {#if team}
    <p class="sub-note">
      {team.matches === 0
        ? "No matches yet."
        : `Plays in ${team.matches} match${team.matches === 1 ? "" : "es"}. Each match keeps its own roster and venue; staff and kit changes reach its exports from now on.`}
    </p>
  {/if}
  {#if error}<p class="error">{error}</p>{/if}
  <div class="actions">
    <button class="btn primary" type="submit" disabled={saving}>Save team</button>
    <button class="btn" type="button" onclick={() => onclose(null)}>Cancel</button>
  </div>
</form>
