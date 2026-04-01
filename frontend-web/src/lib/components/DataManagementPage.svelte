<script lang="ts">
  import { getDatabaseExportUrl, importDatabaseBackup } from "$lib/queryApi";

  let selectedFile = $state<File | null>(null);
  let isConfirmed = $state(false);
  let isLoading = $state(false);
  let error = $state("");
  let success = $state("");

  const exportUrl = getDatabaseExportUrl();

  function handleFileChange(event: Event) {
    const input = event.currentTarget as HTMLInputElement;
    selectedFile = input.files?.[0] ?? null;
    error = "";
    success = "";
  }

  async function handleRestore() {
    error = "";
    success = "";

    if (!selectedFile) {
      error = "请先选择一个 JSON 备份文件。";
      return;
    }

    if (!isConfirmed) {
      error = "请勾选确认框以继续导入操作。";
      return;
    }

    isLoading = true;

    try {
      const response = await importDatabaseBackup(selectedFile);
      success = response.message || "数据库已成功恢复。";
      selectedFile = null;
      isConfirmed = false;
    } catch (restoreError) {
      error = restoreError instanceof Error ? restoreError.message : "恢复过程中出现错误";
    } finally {
      isLoading = false;
    }
  }
</script>

<div class="data-mgmt-grid">
  <div class="bento-card mgmt-card">
    <div class="card-title">
      <i class="ph-fill ph-download-simple"></i> 导出备份
    </div>
    <div class="card-content">
      <p>下载完整的知识库 JSON 备份包。包含：</p>
      <ul class="plain-list" style="margin: 0.75rem 0;">
        <li><i class="ph ph-check-circle"></i> 核心知识词条 (knowledge_entries)</li>
        <li><i class="ph ph-check-circle"></i> 追问历史 (follow_ups)</li>
        <li><i class="ph ph-check-circle"></i> FSRS 学习进度 (learning_progress)</li>
      </ul>
      <p class="small-copy muted-copy" style="margin-bottom: 1.25rem;">注意：备份不包含本地缓存的字典原数据。</p>
      <a href={exportUrl} class="btn-primary" download>
        <i class="ph ph-file-arrow-down"></i> 下载备份文件
      </a>
    </div>
  </div>

  <div class="bento-card mgmt-card danger-zone">
    <div class="card-title">
      <i class="ph-fill ph-upload-simple"></i> 导入与恢复
    </div>
    <div class="card-content">
      <p>从 JSON 备份恢复数据。这将会<strong>覆盖</strong>现有的所有记录，操作不可撤销。</p>
      
      <form class="restore-form" onsubmit={(e) => { e.preventDefault(); handleRestore(); }}>
        <div class="file-input-wrapper" class:has-file={!!selectedFile}>
          <input 
            type="file" 
            id="backup-file"
            accept=".json,application/json" 
            onchange={handleFileChange} 
            class="hidden-input"
          />
          <label for="backup-file" class="file-label">
            <i class="ph ph-file-plus"></i>
            {selectedFile ? selectedFile.name : "点击选择备份文件..."}
          </label>
        </div>

        <label class="confirm-checkbox">
          <input type="checkbox" bind:checked={isConfirmed} />
          <span>我知晓导入将覆盖当前所有词库和学习数据。</span>
        </label>

        {#if error}
          <div class="message-surface error">{error}</div>
        {/if}

        {#if success}
          <div class="message-surface success">{success}</div>
        {/if}

        <button type="submit" class="btn-primary danger-btn" disabled={isLoading || !isConfirmed}>
          {isLoading ? "正在导入数据..." : "确认开始导入"}
        </button>
      </form>
    </div>
  </div>
</div>

<style>
  .data-mgmt-grid {
    display: grid;
    grid-template-columns: repeat(2, 1fr);
    gap: 1.5rem;
    width: 100%;
  }

  .mgmt-card { display: flex; flex-direction: column; height: 100%; }
  .card-content { flex: 1; display: flex; flex-direction: column; }

  .plain-list li { display: flex; align-items: center; gap: 0.5rem; font-size: 0.92rem; color: var(--text-main); }
  .plain-list li i { color: var(--success-text); }

  .restore-form { margin-top: 1.25rem; display: flex; flex-direction: column; gap: 1rem; }

  .file-input-wrapper {
    position: relative;
    border: 2px dashed var(--border-color);
    border-radius: var(--radius-md);
    transition: all var(--transition-fast);
  }

  .file-input-wrapper:hover, .file-input-wrapper.has-file {
    border-color: var(--accent-main);
    background: color-mix(in srgb, var(--accent-main) 5%, transparent);
  }

  .hidden-input { position: absolute; width: 1px; height: 1px; padding: 0; margin: -1px; overflow: hidden; clip: rect(0, 0, 0, 0); border: 0; }

  .file-label {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 0.75rem;
    padding: 2rem 1rem;
    cursor: pointer;
    font-weight: 600;
    color: var(--text-muted);
    text-align: center;
  }

  .has-file .file-label { color: var(--accent-main); }
  .file-label i { font-size: 2rem; }

  .confirm-checkbox { display: flex; align-items: flex-start; gap: 0.75rem; cursor: pointer; font-size: 0.9rem; line-height: 1.4; color: var(--text-muted); }
  .confirm-checkbox input { margin-top: 0.2rem; width: 1.1rem; height: 1.1rem; }

  .danger-zone { border-color: var(--danger-text); }
  .danger-btn { background: var(--danger-bg); color: var(--danger-text); border: 1px solid var(--danger-text); }
  .danger-btn:hover:not(:disabled) { background: var(--danger-text); color: white; }

  @media (max-width: 900px) { .data-mgmt-grid { grid-template-columns: 1fr; } }
</style>
