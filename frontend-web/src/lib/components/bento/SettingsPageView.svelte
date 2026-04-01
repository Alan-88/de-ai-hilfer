<script lang="ts">
  import ServerStatus from "$lib/components/ServerStatus.svelte";
  import DataManagementPage from "$lib/components/DataManagementPage.svelte";

  let { active = false } = $props();
</script>

<header class="page-header">
  <h1>数据与设置</h1>
  <p>管理本地数据存储、数据库备份以及后端 API 服务连接状态。</p>
</header>

<div class="settings-grid">
  <!-- 移除冗余的主题切换，保留核心的服务状态监测 -->
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
  .page-header p { color: var(--text-muted); font-size: 1.1rem; }

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

  @media (max-width: 900px) {
    .settings-grid { grid-template-columns: 1fr; }
  }
</style>
