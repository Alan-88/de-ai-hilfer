<script lang="ts">
  import type { ActionModelOption } from "$lib/ai/actionModels";

  let {
    message,
    options = [],
    selectedKey = "",
    disabled = false,
    onModelChange,
    onRetryOriginal,
    onRetrySelected,
  } = $props<{
    message: string;
    options?: ActionModelOption[];
    selectedKey?: string;
    disabled?: boolean;
    onModelChange: (key: string) => void;
    onRetryOriginal: () => void;
    onRetrySelected: () => void;
  }>();

  const providers = $derived(Array.from(new Set(options.map((option: ActionModelOption) => option.provider_name))));
  const selectedProvider = $derived(
    options.find((option: ActionModelOption) => option.key === selectedKey)?.provider_name ?? providers[0] ?? ""
  );
  const providerModels = $derived(
    options.filter((option: ActionModelOption) => option.provider_name === selectedProvider)
  );

  function selectProvider(providerName: string) {
    const firstModel = options.find((option: ActionModelOption) => option.provider_name === providerName);
    if (firstModel) onModelChange(firstModel.key);
  }
</script>

<div class="bento-card retry-card" role="alert">
  <div class="failure-copy">
    <span class="failure-icon"><i class="ph-fill ph-warning-circle"></i></span>
    <div>
      <strong>分析没有完成</strong>
      <p>{message}</p>
    </div>
  </div>

  <div class="retry-actions">
    <button class="btn-secondary" onclick={onRetryOriginal} {disabled}>
      <i class="ph ph-arrows-clockwise"></i>
      按原设置重试
    </button>

    {#if options.length > 0}
      <div class="model-retry">
        <select
          aria-label="重试渠道"
          title="渠道"
          value={selectedProvider}
          onchange={(event) => selectProvider(event.currentTarget.value)}
          {disabled}
        >
          {#each providers as provider}
            <option value={provider}>{provider}</option>
          {/each}
        </select>
        <select
          aria-label="重试模型"
          title="模型"
          value={selectedKey}
          onchange={(event) => onModelChange(event.currentTarget.value)}
          {disabled}
        >
          {#each providerModels as option}
            <option value={option.key}>{option.model_id}</option>
          {/each}
        </select>
        <button class="btn-primary" onclick={onRetrySelected} {disabled}>
          换模型重试
        </button>
      </div>
    {/if}
  </div>
</div>

<style>
  .retry-card { display: grid; gap: 1.2rem; padding: 1.35rem; }
  .failure-copy { display: flex; align-items: flex-start; gap: 0.9rem; }
  .failure-icon { display: grid; flex: 0 0 2.25rem; width: 2.25rem; height: 2.25rem; place-items: center; border-radius: 50%; background: var(--danger-bg); color: var(--danger-text); }
  .failure-icon i { font-size: 1.2rem; }
  .failure-copy > div { display: grid; gap: 0.3rem; min-width: 0; }
  .failure-copy strong { font-size: 1.02rem; }
  .failure-copy p { color: var(--text-muted); line-height: 1.55; overflow-wrap: anywhere; }
  .retry-actions { display: flex; align-items: center; justify-content: space-between; gap: 0.8rem; }
  .retry-actions button { min-height: 42px; white-space: nowrap; }
  .model-retry { display: flex; align-items: center; gap: 0.55rem; min-width: 0; }
  .model-retry select { min-width: 0; height: 42px; padding: 0 2rem 0 0.75rem; border: 1px solid var(--border-color); border-radius: var(--radius-md); background: var(--bg-color); color: var(--text-main); font-weight: 700; }
  .model-retry select:first-child { max-width: 150px; }
  .model-retry select:nth-child(2) { max-width: 220px; }

  @media (max-width: 760px) {
    .retry-actions, .model-retry { align-items: stretch; flex-direction: column; }
    .model-retry { width: 100%; }
    .model-retry select, .model-retry select:first-child, .model-retry select:nth-child(2), .retry-actions button { width: 100%; max-width: none; }
  }
</style>
