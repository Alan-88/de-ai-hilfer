<script lang="ts">
  import ServerStatus from "$lib/components/ServerStatus.svelte";
  import DataManagementPage from "$lib/components/DataManagementPage.svelte";
  import AiModelSettingsPanel from "$lib/components/bento/AiModelSettingsPanel.svelte";
  import { theme } from "$lib/stores/ui";

  let { active = false } = $props();
</script>

<header class="page-header">
  <h1>数据与设置</h1>
</header>

<div class="settings-grid">
  <section class="bento-card setting-section mobile-appearance" aria-label="外观设置">
    <div class="card-title"><i class="ph-fill ph-palette"></i> 外观</div>
    <div class="theme-options">
      <button class:chosen={$theme === 'canvas'} aria-label="浅色" aria-pressed={$theme === 'canvas'} onclick={() => $theme = 'canvas'}><i class="ph ph-sun" aria-hidden="true"></i> 浅色</button>
      <button class:chosen={$theme === 'canvas-dark'} aria-label="深色" aria-pressed={$theme === 'canvas-dark'} onclick={() => $theme = 'canvas-dark'}><i class="ph ph-moon" aria-hidden="true"></i> 深色</button>
    </div>
  </section>
  <AiModelSettingsPanel />

  <div class="bento-card setting-section">
    <div class="card-title"><i class="ph-fill ph-broadcast"></i> 后端服务状态</div>
    <div class="status-wrapper">
      <ServerStatus />
    </div>
    <p class="small-copy muted-copy" style="margin-top: 0.5rem;">
      如果无法连接后端，请检查网络环境或 Docker 容器运行状态。
    </p>
  </div>

  <div class="card-full">
    <DataManagementPage />
  </div>
</div>

<style>
  .page-header { width: 100%; margin-bottom: 2.5rem; }
  .page-header h1 { font-size: 2.25rem; font-weight: 800; margin-bottom: 0.5rem; color: var(--text-main); }

  .settings-grid {
    display: grid;
    grid-template-columns: 1fr; /* 简化为单列布局或根据内容自动调整 */
    gap: 1.5rem;
    width: 100%;
  }

  .setting-section {
    padding: 1.5rem;
    display: flex;
    flex-direction: column;
    gap: 1rem;
  }

  .status-wrapper {
    padding: 0.5rem 0;
  }

  .card-full { width: 100%; }
  .mobile-appearance { display: none; }
  .theme-options { display: flex; gap: 0.75rem; }
  .theme-options button { flex: 1; min-height: 44px; display: flex; align-items: center; justify-content: center; gap: 0.5rem; border-radius: 12px; border: 1px solid var(--border-color); background: var(--control-bg); color: var(--text-main); }
  .theme-options button.chosen { border-color: var(--accent-main); background: var(--btn-secondary); color: var(--accent-main); }
  @media (max-width: 768px) {
    .mobile-appearance { display: flex; }
  }

  @media (max-width: 900px) {
    .settings-grid { grid-template-columns: 1fr; }
  }
</style>
