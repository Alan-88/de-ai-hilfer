<script lang="ts">
  import { renderMarkdownHtml } from "$lib/analysis/structuredAnalysis";
  import { streamFollowUp } from "$lib/streamApi";
  import type { FollowUpItem, QualityMode } from "$lib/types";

  let { 
    entryId, 
    disabled = false, 
    history = [], 
    onnewFollowUp 
  } = $props<{
    entryId: number;
    disabled?: boolean;
    history?: FollowUpItem[];
    onnewFollowUp?: (item: FollowUpItem) => void;
  }>();

  let question = $state("");
  let pendingAnswer = $state("");
  let pendingModel = $state("");
  let activeQualityMode = $state<QualityMode>("default");
  let isLoading = $state(false);
  let error = $state("");
  let textareaRef = $state<HTMLTextAreaElement | null>(null);

  // 自动调整高度逻辑
  $effect(() => {
    if (question !== undefined && textareaRef) {
      textareaRef.style.height = 'auto';
      textareaRef.style.height = textareaRef.scrollHeight + 'px';
    }
  });

  async function submit(qualityMode: QualityMode = "default") {
    if (!question.trim() || isLoading) return;

    isLoading = true;
    error = "";
    pendingAnswer = "";
    pendingModel = "";
    activeQualityMode = qualityMode;

    try {
      const response = await new Promise<{ follow_up: FollowUpItem }>((resolve, reject) => {
        void streamFollowUp(
          { entry_id: entryId, question, quality_mode: qualityMode },
          {
            onMeta: (payload) => {
              pendingModel = payload.model;
            },
            onDelta: (payload) => {
              pendingAnswer = `${pendingAnswer}${payload.delta}`;
            },
            onComplete: resolve,
            onError: (payload) => reject(new Error(payload.message)),
          }
        ).catch(reject);
      });

      if (onnewFollowUp) onnewFollowUp(response.follow_up);
      question = "";
      pendingAnswer = "";
      pendingModel = "";
    } catch (submitError) {
      error = submitError instanceof Error ? submitError.message : "追问失败";
    } finally {
      isLoading = false;
    }
  }

  const pendingHtml = $derived(pendingAnswer ? renderMarkdownHtml(pendingAnswer) : "");
</script>

<div class="card-title"><i class="ph-fill ph-chat-circle-dots"></i> 继续追问</div>

<div class="form-grid">
  <textarea
    bind:this={textareaRef}
    bind:value={question}
    class="auto-expand-input"
    placeholder="例如：这个词和 helfen 的区别是什么？"
    disabled={disabled || isLoading}
    rows="2"
  ></textarea>

  {#if error}
    <div class="message-surface error">{error}</div>
  {/if}

  {#if pendingAnswer}
    <div class="surface-card">
      <p class="small-copy">
        {activeQualityMode === "pro" ? "Pro 追问生成中" : "追问生成中"}
        {#if pendingModel}
          · {pendingModel}
        {/if}
      </p>
      <div class="markdown-compact">
        {@html pendingHtml}
      </div>
    </div>
  {/if}

  <div class="inline-actions">
    <button
      class="btn-secondary"
      onclick={() => submit("default")}
      disabled={disabled || isLoading || !question.trim()}
    >
      <i class="ph ph-paper-plane-right"></i>
      {isLoading && activeQualityMode === "default" ? "生成中" : "发送追问"}
    </button>
    <button
      class="btn-primary"
      onclick={() => submit("pro")}
      disabled={disabled || isLoading || !question.trim()}
    >
      <i class="ph-fill ph-sparkle"></i>
      {isLoading && activeQualityMode === "pro" ? "Pro 生成中" : "Pro 增强"}
    </button>
  </div>

  {#if history.length > 0}
    <div class="follow-up-history">
      {#each history as item (item.id)}
        <article class="surface-card">
          <p><strong>你问：</strong>{item.question}</p>
          <div class="markdown-compact">
            {@html renderMarkdownHtml(item.answer)}
          </div>
        </article>
      {/each}
    </div>
  {/if}
</div>

<style>
  .form-grid { display: flex; flex-direction: column; gap: 1rem; }
  
  /* 优化后的输入框：自动扩展、描边、取消拉伸 */
  .auto-expand-input {
    width: 100%;
    padding: 1rem;
    border-radius: var(--radius-md);
    background: var(--bg-color);
    border: 1px solid var(--border-color);
    color: var(--text-main);
    font-size: 1rem;
    line-height: 1.5;
    resize: none;
    overflow: hidden;
    transition: border-color 0.2s, box-shadow 0.2s;
  }

  .auto-expand-input:focus {
    border-color: var(--accent-main);
    box-shadow: 0 0 0 3px color-mix(in srgb, var(--accent-main) 15%, transparent);
    background: var(--card-bg);
  }

  .follow-up-history { margin-top: 1.25rem; display: flex; flex-direction: column; gap: 0.75rem; }
  .follow-up-history article { padding: 1.25rem; border: 1px solid var(--border-color); border-radius: var(--radius-sm); background: var(--bg-color); }
  .follow-up-history p { margin-bottom: 0.5rem; color: var(--accent-main); font-weight: 700; font-size: 1rem; }
  .inline-actions { display: flex; gap: 0.75rem; }
</style>
