<script lang="ts">
  import { createEventDispatcher } from "svelte";
  import MarkdownRenderer from "$lib/components/MarkdownRenderer.svelte";
  import { streamFollowUp } from "$lib/streamApi";
  import type { FollowUpCreateResponse, FollowUpItem, QualityMode } from "$lib/types";

  export let entryId: number;
  export let disabled = false;

  let question = "";
  let isLoading = false;
  let pendingAnswer = "";
  let pendingModel = "";
  let activeQualityMode: QualityMode = "default";
  let error = "";

  const dispatch = createEventDispatcher<{ newFollowUp: FollowUpItem }>();

  async function submit(qualityMode: QualityMode = "default") {
    if (!question.trim()) return;

    isLoading = true;
    error = "";
    pendingAnswer = "";
    pendingModel = "";
    activeQualityMode = qualityMode;

    try {
      const response = await new Promise<FollowUpCreateResponse>((resolve, reject) => {
        void streamFollowUp(
          {
            entry_id: entryId,
            question,
            quality_mode: qualityMode,
          },
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

      dispatch("newFollowUp", response.follow_up);
      pendingAnswer = "";
      pendingModel = "";
      question = "";
    } catch (err) {
      error = err instanceof Error ? err.message : "追问失败";
    } finally {
      isLoading = false;
    }
  }
</script>

<section class="follow-up">
  <div class="title-row">
    <h3>继续追问</h3>
    <span>围绕这条分析继续深挖</span>
  </div>

  <textarea
    bind:value={question}
    rows="3"
    placeholder="例如：这个词和 helfen 的区别是什么？"
    disabled={isLoading}
  ></textarea>

  {#if error}
    <p class="error">{error}</p>
  {/if}

  {#if pendingAnswer}
    <div class="preview-card">
      <p class="preview-label">
        {activeQualityMode === "pro" ? "Pro 追问生成中" : "追问生成中"}
        {#if pendingModel}
          · {pendingModel}
        {/if}
      </p>
      <MarkdownRenderer markdownContent={pendingAnswer} />
    </div>
  {/if}

  <div class="actions">
    <button on:click={() => submit("default")} disabled={disabled || isLoading || !question.trim()}>
      {isLoading && activeQualityMode === "default" ? "思考中..." : "发送追问"}
    </button>
    <button
      class="pro-button"
      on:click={() => submit("pro")}
      disabled={disabled || isLoading || !question.trim()}
    >
      {isLoading && activeQualityMode === "pro" ? "Pro 思考中..." : "高质量追问"}
    </button>
  </div>
</section>

<style>
  .follow-up {
    margin-top: 2rem;
    padding-top: 1.5rem;
    border-top: 1px solid var(--border);
  }

  .title-row {
    margin-bottom: 0.75rem;
  }

  .title-row h3 {
    margin: 0;
    font-family: var(--font-ui);
    color: var(--accent-strong);
  }

  .title-row span {
    display: block;
    margin-top: 0.2rem;
    color: var(--muted);
    font-size: 0.95rem;
  }

  textarea {
    width: 100%;
    border: 1px solid var(--border);
    border-radius: 0.9rem;
    padding: 0.85rem 1rem;
    background: #fff;
    resize: vertical;
    min-height: 6rem;
  }

  .error {
    color: var(--danger);
    margin: 0.5rem 0 0;
  }

  .actions {
    display: flex;
    justify-content: flex-end;
    gap: 0.75rem;
    margin-top: 0.85rem;
  }

  .preview-card {
    margin-top: 1rem;
    padding: 0.95rem 1rem;
    border-radius: 1rem;
    background: color-mix(in srgb, var(--panel-alt) 88%, white 12%);
  }

  .preview-label {
    margin: 0 0 0.75rem;
    color: var(--muted);
    font-family: var(--font-ui);
    font-size: 0.86rem;
  }

  button {
    border: 0;
    border-radius: 999px;
    background: var(--accent);
    color: #fff;
    padding: 0.75rem 1.15rem;
    font-family: var(--font-ui);
    font-weight: 600;
  }

  .pro-button {
    background: color-mix(in srgb, var(--accent-strong) 82%, black 18%);
  }

  button:disabled {
    opacity: 0.55;
    cursor: not-allowed;
  }
</style>
