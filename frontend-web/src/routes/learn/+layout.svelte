<script lang="ts">
  import LearningModeTitle from "$lib/components/learning/LearningModeTitle.svelte";

  let { children } = $props();
</script>

<div class="learning-mode-shell">
  <LearningModeTitle />
  {@render children()}
</div>

<style>
  :global(.page-container:has(.learning-mode-shell)) { max-width: 1160px; }
  .learning-mode-shell { width: 100%; }
</style>
