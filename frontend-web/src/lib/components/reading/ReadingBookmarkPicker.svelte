<script lang="ts">
  import type { DBSuggestion } from "$lib/types";

  interface Props {
    surface: string;
    x: number;
    y: number;
    suggestions: DBSuggestion[];
    loading: boolean;
    saving: boolean;
    error: string;
    onChoose: (suggestion: DBSuggestion) => void;
    onClose: () => void;
  }

  let { surface, x, y, suggestions, loading, saving, error, onChoose, onClose }: Props = $props();
</script>

<svelte:window onkeydown={(event) => { if (event.key === "Escape") onClose(); }} />

<div
  class="reading-mark-picker"
  role="dialog"
  tabindex="-1"
  aria-label={`标记 ${surface}`}
  style:left={`${x}px`}
  style:top={`${y}px`}
  onpointerdown={(event) => event.stopPropagation()}
>
  <header>
    <strong>{surface}</strong>
    <button aria-label="关闭" onclick={onClose}><i class="ph ph-x"></i></button>
  </header>

  {#if loading || saving}
    <div class="picker-state"><i class="ph ph-spinner-gap spin"></i><span>{saving ? "正在标记" : "正在查找"}</span></div>
  {:else if error}
    <div class="picker-state error"><i class="ph ph-warning-circle"></i><span>{error}</span></div>
  {:else if suggestions.length === 0}
    <div class="picker-state"><span>没有找到候选</span></div>
  {:else}
    <div class="candidate-list">
      {#each suggestions as suggestion}
        <button onclick={() => onChoose(suggestion)}>
          <span><strong>{suggestion.query_text}</strong><small>{suggestion.preview}</small></span>
          {#if suggestion.entry_id > 0}
            <i class="ph ph-arrow-right"></i>
          {:else}
            <em>生成</em>
          {/if}
        </button>
      {/each}
    </div>
  {/if}
</div>

<style>
  .reading-mark-picker { position: fixed; z-index: 540; width: min(310px, calc(100vw - 24px)); overflow: hidden; border: 1px solid var(--border-color); border-radius: var(--radius-md); background: color-mix(in srgb, var(--card-bg) 97%, transparent); box-shadow: var(--shadow-hover); backdrop-filter: blur(14px); color: var(--text-main); }
  header { display: flex; min-height: 44px; align-items: center; justify-content: space-between; gap: 0.75rem; padding: 0 0.55rem 0 0.85rem; border-bottom: 1px solid var(--border-color); }
  header strong { overflow: hidden; font-size: 0.88rem; text-overflow: ellipsis; white-space: nowrap; }
  header button { display: grid; width: 30px; height: 30px; flex: 0 0 auto; place-items: center; border-radius: 50%; background: transparent; color: var(--text-muted); }
  header button:hover, header button:focus-visible { background: var(--btn-secondary); color: var(--text-main); outline: none; }
  .candidate-list { display: grid; max-height: min(310px, 55vh); overflow-y: auto; padding: 0.38rem; }
  .candidate-list > button { display: flex; min-height: 56px; align-items: center; justify-content: space-between; gap: 0.75rem; padding: 0.55rem 0.65rem; border-radius: 9px; background: transparent; color: var(--text-main); text-align: left; }
  .candidate-list > button:hover, .candidate-list > button:focus-visible { background: var(--btn-secondary); outline: none; }
  .candidate-list span { display: grid; min-width: 0; gap: 0.15rem; }
  .candidate-list strong, .candidate-list small { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .candidate-list strong { font-size: 0.84rem; }
  .candidate-list small { color: var(--text-muted); font-size: 0.7rem; }
  .candidate-list > button > i { flex: 0 0 auto; color: var(--text-muted); }
  .candidate-list em { flex: 0 0 auto; padding: 0.24rem 0.42rem; border-radius: var(--radius-full); background: color-mix(in srgb, var(--accent-main) 10%, transparent); color: var(--accent-main); font-size: 0.62rem; font-style: normal; font-weight: 850; }
  .picker-state { display: flex; min-height: 78px; align-items: center; justify-content: center; gap: 0.5rem; padding: 0.85rem; color: var(--text-muted); font-size: 0.76rem; }
  .picker-state.error { align-items: flex-start; justify-content: flex-start; color: var(--danger-text, #b8453f); line-height: 1.5; }
</style>
