<script lang="ts">
  import { onMount } from "svelte";
  import { getServerStatus } from "$lib/queryApi";
  import type { StatusResponse } from "$lib/types";

  type StatusKind = "unknown" | "checking" | "ok" | "error";

  let statusKind = $state<StatusKind>("unknown");
  let details = $state<StatusResponse | null>(null);
  let message = $state("尚未检查");

  async function checkStatus() {
    statusKind = "checking";
    message = "正在连接后端服务...";

    try {
      details = await getServerStatus();
      if (details.status === "ok" && details.db_status === "ok") {
        statusKind = "ok";
        message = "系统运行正常，数据库已连接。";
      } else {
        statusKind = "error";
        message = "服务响应异常，请检查后端日志。";
      }
    } catch (error) {
      statusKind = "error";
      message = error instanceof Error ? error.message : "无法连接到后端 API 服务";
    }
  }

  function statusLabel(kind: StatusKind) {
    switch (kind) {
      case "checking": return "检查中";
      case "ok": return "在线";
      case "error": return "异常";
      default: return "未知";
    }
  }

  onMount(() => {
    void checkStatus();
  });
</script>

<div class="status-box surface-card">
  <div class="status-main">
    <span class="status-dot" class:ok={statusKind === 'ok'} class:checking={statusKind === 'checking'} class:error={statusKind === 'error'}></span>
    <div class="info">
      <div class="label">API 服务状态 · {statusLabel(statusKind)}</div>
      <div class="msg">{message}</div>
    </div>
  </div>
  <button type="button" class="btn-secondary small-btn" onclick={checkStatus} disabled={statusKind === 'checking'}>
    <i class="ph ph-arrows-clockwise" class:spinning={statusKind === 'checking'}></i>
    {statusKind === 'checking' ? "刷新中" : "刷新状态"}
  </button>
</div>

<style>
  .status-box {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 1.5rem;
    padding: 1.25rem;
  }

  .status-main {
    display: flex;
    align-items: center;
    gap: 1rem;
  }

  .status-dot {
    width: 0.75rem;
    height: 0.75rem;
    border-radius: 50%;
    background: var(--text-muted);
    flex-shrink: 0;
  }

  .status-dot.ok { background: var(--success-text); box-shadow: 0 0 8px var(--success-text); }
  .status-dot.checking { background: var(--warning-text); animation: pulse 1s infinite; }
  .status-dot.error { background: var(--danger-text); }

  @keyframes pulse {
    0% { opacity: 0.5; }
    50% { opacity: 1; }
    100% { opacity: 0.5; }
  }

  .info { display: flex; flex-direction: column; gap: 0.2rem; }
  .label { font-size: 0.85rem; font-weight: 700; color: var(--text-muted); text-transform: uppercase; }
  .msg { font-size: 0.95rem; color: var(--text-main); font-weight: 600; }

  .small-btn { padding: 0.5rem 1rem; font-size: 0.85rem; }

  .spinning {
    animation: spin 1s linear infinite;
  }

  @keyframes spin {
    from { transform: rotate(0deg); }
    to { transform: rotate(360deg); }
  }

  @media (max-width: 600px) {
    .status-box { flex-direction: column; align-items: flex-start; gap: 1rem; }
    button { width: 100%; }
  }
</style>
