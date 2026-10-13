<script lang="ts">
  import { onDestroy } from "svelte";
  import { renderFollowUpHtml } from "$lib/analysis/followUpMarkdown";
  import { editFollowUp, deleteFollowUp } from "$lib/followUpApi";
  import { streamFollowUp } from "$lib/streamApi";
  import type { AiModelOverride, FollowUpItem } from "$lib/types";

  let {entryId, history, disabled = false, modelOverride = null, onchange} = $props<{
    entryId: number; history: FollowUpItem[]; disabled?: boolean;
    modelOverride?: AiModelOverride | null; onchange: (items: FollowUpItem[]) => void;
  }>();
  let busyId = $state<number | null>(null);
  let editingId = $state<number | null>(null);
  let question = $state("");
  let answer = $state("");
  let error = $state("");
  let confirmDeleteId = $state<number | null>(null);
  let controller: AbortController | null = null;
  onDestroy(() => controller?.abort());
  $effect(() => {
    entryId;
    return () => {controller?.abort(); editingId = null; confirmDeleteId = null; error = "";};
  });

  function startEdit(item: FollowUpItem) {
    editingId = item.id; question = item.question; answer = item.answer; error = "";
    confirmDeleteId = null;
  }

  async function operate(item: FollowUpItem, action: "save" | "delete" | "retry") {
    if (busyId !== null || disabled) return;
    busyId = item.id; error = "";
    const originalEntry = entryId;
    try {
      if (action === "delete") {
        await deleteFollowUp(item.id);
        if (entryId === originalEntry) onchange(history.filter((record: FollowUpItem) => record.id !== item.id));
      } else {
        let saved: FollowUpItem;
        if (action === "save") saved = await editFollowUp(item, question, answer);
        else {
          controller = new AbortController();
          let response: FollowUpItem | undefined;
          await streamFollowUp({entry_id: entryId, question: item.question, retry_id: item.id, model_override: modelOverride}, {
            signal: controller.signal, onComplete: value => response = value.follow_up,
            onError: value => { throw new Error(value.message); },
          });
          if (!response) throw new Error("追问未完成，请重试；原回答已保留");
          saved = response;
        }
        if (entryId === originalEntry) onchange(history.map((record: FollowUpItem) => record.id === saved.id ? saved : record));
      }
      editingId = null; confirmDeleteId = null;
    } catch (cause) {if (entryId === originalEntry) error = cause instanceof Error ? cause.message : "操作失败";}
    finally {busyId = null; controller = null;}
  }
</script>

<div class="follow-up-history">
  {#each history as item (item.id)}
    <article class="surface-card follow-up-item">
      {#if editingId === item.id}
        <label>问题<textarea bind:value={question} rows="2" maxlength="4000" disabled={disabled || busyId !== null}></textarea></label>
        <label>回答（Markdown）<textarea class="answer-editor" bind:value={answer} rows="8" maxlength="100000" disabled={disabled || busyId !== null}></textarea></label>
        <div class="follow-up-actions">
          <button class="btn-primary" onclick={() => operate(item, "save")} disabled={disabled || busyId !== null || !question.trim() || !answer.trim()}>保存</button>
          <button class="btn-secondary" onclick={() => editingId = null} disabled={busyId !== null}>取消</button>
        </div>
      {:else}
        <p class="follow-up-question"><strong>你问：</strong>{item.question}</p>
        <div class="markdown-compact follow-up-answer">{@html renderFollowUpHtml(item.answer)}</div>
        <div class="follow-up-actions">
          <button class="btn-secondary" onclick={() => startEdit(item)} disabled={disabled || busyId !== null}>编辑</button>
          <button class="btn-secondary" onclick={() => operate(item, "retry")} disabled={disabled || busyId !== null}>{busyId === item.id ? "处理中…" : "重试"}</button>
          <button class="btn-secondary danger-text" onclick={() => confirmDeleteId = item.id} disabled={disabled || busyId !== null}>删除</button>
        </div>
      {/if}
      {#if confirmDeleteId === item.id}
        <div class="delete-confirm" role="group" aria-label="确认删除追问">
          <span>删除这条追问？</span>
          <button class="btn-secondary danger-text" onclick={() => operate(item, "delete")} disabled={busyId !== null}>确认删除</button>
          <button class="btn-secondary" onclick={() => confirmDeleteId = null} disabled={busyId !== null}>取消</button>
        </div>
      {/if}
    </article>
  {/each}
  {#if error}<div class="message-surface error" role="alert">{error}</div>{/if}
</div>

<style>
  .follow-up-history { display: flex; flex-direction: column; gap: 12px; min-width: 0; }
  .follow-up-item { padding: 20px; min-width: 0; }
  .follow-up-question { color: var(--accent-main); margin-bottom: 12px; overflow-wrap: anywhere; }
  .follow-up-answer { overflow-wrap: anywhere; }
  .follow-up-answer :global(pre) { overflow-x: auto; padding: 8px; }
  .follow-up-answer :global(table) { display: block; max-width: 100%; overflow-x: auto; border-collapse: collapse; }
  .follow-up-answer :global(th), .follow-up-answer :global(td) { padding: 6px 10px; border: 1px solid var(--border-color); }
  .follow-up-actions, .delete-confirm { display: flex; flex-wrap: wrap; align-items: center; gap: 8px; margin-top: 14px; }
  .follow-up-actions button, .delete-confirm button { padding: 7px 16px; min-height: 36px; font-size: 12px; white-space: nowrap; }
  label { display: grid; gap: 6px; margin-bottom: 12px; font-size: 13px; color: var(--text-muted); }
  textarea { width: 100%; resize: vertical; padding: 10px; border: 1px solid var(--border-color); border-radius: 12px; background: var(--bg-color); color: var(--text-main); font: inherit; line-height: 1.6; }
  .answer-editor { min-height: 160px; }
  .danger-text { color: var(--danger-text); }
  @media (max-width: 600px) {
    .follow-up-item { padding: 12px; font-size: 13px; }
    .follow-up-answer { line-height: 1.6; }
    .follow-up-answer :global(p), .follow-up-answer :global(ul), .follow-up-answer :global(ol) { margin-bottom: 8px; }
  }
</style>
