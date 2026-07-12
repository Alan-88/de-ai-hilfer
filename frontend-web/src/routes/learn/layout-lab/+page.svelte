<script lang="ts">
  import LearningCardPrototype from "$lib/components/learning-layout-lab/LearningCardPrototype.svelte";

  let revealed = $state(false);
</script>

<svelte:head><title>学习卡片预览 | De-AI-Hilfer</title></svelte:head>

<div class="preview-shell">
  <header class="page-heading">
    <div>
      <h1>沉浸复习</h1>
    </div>
    <button onclick={() => revealed = !revealed}>
      <i class={revealed ? "ph ph-eye-slash" : "ph ph-eye"}></i>
      {revealed ? "查看回忆态" : "查看答案态"}
    </button>
  </header>
  <LearningCardPrototype bind:revealed />
</div>

<style>
  .preview-shell { width: min(1040px, calc(100vw - 304px)); margin-left: 50%; transform: translateX(-50%); }
  .page-heading { display: flex; align-items: flex-end; justify-content: space-between; gap: 1.5rem; margin-bottom: 1.2rem; }
  .page-heading h1 { font-size: 1.75rem; font-weight: 800; }
  .page-heading button { display: inline-flex; align-items: center; gap: 0.45rem; min-height: 38px; padding: 0 0.95rem; border: 1px solid var(--border-color); border-radius: var(--radius-full); background: var(--card-bg); color: var(--text-main); font-weight: 750; }
  @media (max-width: 1100px) { .preview-shell { width: 100%; margin-left: 0; transform: none; } }
  @media (max-width: 600px) { .page-heading { align-items: flex-start; flex-direction: column; } }
</style>
