<script lang="ts">
  import Icon from "./Icon.svelte";
</script>

<!-- Placeholder until video work starts (PRD phase 4). -->
<section class="video" aria-label="Video">
  <div class="screen">
    <div>
      <svg
        viewBox="0 0 180 90"
        fill="none"
        stroke="currentColor"
        stroke-width="1.5"
        aria-hidden="true"
        ><rect x="10" y="15" width="160" height="60" rx="2" /><path
          d="M90 8v74M63 15v60M117 15v60"
        /><circle cx="90" cy="8" r="2" /><circle cx="90" cy="82" r="2" /></svg
      >
      <p>Match video comes in a later phase.</p>
    </div>
  </div>
  <div class="transport">
    <button class="icon-btn" type="button" disabled aria-label="Back 5 seconds"
      ><Icon name="rewind" /></button
    >
    <button class="icon-btn" type="button" disabled aria-label="Play"><Icon name="play" /></button>
    <button class="icon-btn" type="button" disabled aria-label="Forward 5 seconds"
      ><Icon name="forward" /></button
    >
    <span class="timecode">--:--</span>
    <div class="scrub" aria-hidden="true"><span style="width: 0"></span></div>
    <span class="rate">1×</span>
  </div>
</section>
