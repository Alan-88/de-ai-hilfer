<script lang="ts">
  import MarkdownRenderer from "$lib/components/MarkdownRenderer.svelte";
  import QualityRating from "$lib/components/QualityRating.svelte";
  import { buildLearningEnhancements } from "$lib/learningEnhance";
  import type { LearningSessionWord, ReviewQuality } from "$lib/types";

  export let wordData: LearningSessionWord;
  export let onReviewed: (word: LearningSessionWord, quality: ReviewQuality) => void = () => {};

  let showingAnswer = false;
  let isSubmitting = false;
  let exampleIndex = 0;
  let selectedQuizIndex: number | null = null;

  function formatDate(value?: string | null) {
    if (!value) return "未复习";
    return new Intl.DateTimeFormat("zh-CN", {
      month: "numeric",
      day: "numeric",
      hour: "2-digit",
      minute: "2-digit"
    }).format(new Date(value));
  }

  function learningStateLabel(state?: number | null) {
    switch (state) {
      case 1:
        return "学习中";
      case 2:
        return "复习期";
      case 3:
        return "重新学习";
      default:
        return "新词";
    }
  }

  async function handleRating(quality: ReviewQuality) {
    if (isSubmitting) return;
    isSubmitting = true;
    await onReviewed(wordData, quality);
    isSubmitting = false;
  }

  function nextExample() {
    if (!enhancement.examples.length) return;
    exampleIndex = (exampleIndex + 1) % enhancement.examples.length;
  }

  $: if (wordData) {
    showingAnswer = false;
    isSubmitting = false;
    exampleIndex = 0;
    selectedQuizIndex = null;
  }

  $: enhancement = buildLearningEnhancements(wordData.analysis_markdown, wordData.query_text);
  $: activeExample = enhancement.examples[exampleIndex] ?? null;
  $: quizCorrect = enhancement.quiz && selectedQuizIndex !== null
    ? selectedQuizIndex === enhancement.quiz.answerIndex
    : null;
</script>

<article class="card-shell">
  <div class="meta">
    <span class="pill">今日学习</span>
    <span class="subtle">{learningStateLabel(wordData.progress?.state)} · 剩余估计 {wordData.repetitions_left} 次</span>
  </div>

  <h2>{wordData.query_text}</h2>

  <div class="stats-grid">
    <article>
      <span>复习次数</span>
      <strong>{wordData.progress?.review_count ?? 0}</strong>
    </article>
    <article>
      <span>下次到期</span>
      <strong>{formatDate(wordData.progress?.next_review_at)}</strong>
    </article>
    <article>
      <span>上次复习</span>
      <strong>{formatDate(wordData.progress?.last_reviewed_at)}</strong>
    </article>
  </div>

  {#if !showingAnswer}
    <div class="recall-box">
      <p>先别急着翻答案，试着先说出：</p>
      <ul>
        <li>1 个核心义项</li>
        <li>1 个关键形式或语法点</li>
        <li>1 个你最容易混淆的地方</li>
      </ul>
    </div>
    <div class="front-actions">
      <button class="primary" on:click={() => (showingAnswer = true)}>查看答案</button>
    </div>
  {:else}
    <div class="answer">
      <MarkdownRenderer markdownContent={wordData.analysis_markdown} />
    </div>

    {#if enhancement.insights.length > 0}
      <section class="insight-box">
        <h3>学习洞察</h3>
        <ul>
          {#each enhancement.insights as insight}
            <li>{insight}</li>
          {/each}
        </ul>
      </section>
    {/if}

    {#if activeExample}
      <section class="example-box">
        <div class="example-head">
          <h3>动态例句</h3>
          {#if enhancement.examples.length > 1}
            <button class="ghost" on:click={nextExample}>换一个</button>
          {/if}
        </div>
        <p class="de-example">{activeExample.de}</p>
        {#if activeExample.zh}
          <p class="zh-example">{activeExample.zh}</p>
        {/if}
      </section>
    {/if}

    {#if enhancement.quiz}
      <section class="quiz-box">
        <h3>快速小测</h3>
        <p>{enhancement.quiz.question}</p>
        <div class="quiz-options">
          {#each enhancement.quiz.options as option, idx}
            <button
              class:selected={selectedQuizIndex === idx}
              disabled={selectedQuizIndex !== null}
              on:click={() => (selectedQuizIndex = idx)}
            >
              {option}
            </button>
          {/each}
        </div>
        {#if selectedQuizIndex !== null}
          <p class={quizCorrect ? "quiz-feedback ok" : "quiz-feedback bad"}>
            {quizCorrect ? "回答正确。" : "回答不正确。"} {enhancement.quiz.explanation}
          </p>
        {/if}
      </section>
    {/if}

    <QualityRating disabled={isSubmitting} onRating={handleRating} />
  {/if}
</article>

<style>
  .card-shell {
    background: color-mix(in srgb, var(--panel) 92%, white 8%);
    border: 1px solid var(--border);
    box-shadow: var(--shadow);
    border-radius: 1.6rem;
    padding: 1.4rem;
  }

  .meta {
    display: flex;
    justify-content: space-between;
    gap: 1rem;
    margin-bottom: 1rem;
  }

  .pill {
    background: var(--panel-alt);
    color: var(--accent-strong);
    border-radius: 999px;
    padding: 0.3rem 0.7rem;
    font-family: var(--font-ui);
    font-size: 0.84rem;
    font-weight: 700;
  }

  .subtle {
    color: var(--muted);
    font-family: var(--font-ui);
    font-size: 0.9rem;
  }

  h2 {
    margin: 0 0 1rem;
    font-size: clamp(2rem, 5vw, 3.6rem);
    line-height: 1;
  }

  .stats-grid {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: 0.7rem;
    margin-bottom: 1rem;
  }

  .stats-grid article {
    background: var(--panel-alt);
    border-radius: 1rem;
    padding: 0.8rem 0.9rem;
  }

  .stats-grid span {
    display: block;
    margin-bottom: 0.3rem;
    color: var(--muted);
    font-family: var(--font-ui);
    font-size: 0.8rem;
  }

  .stats-grid strong {
    font-size: 0.98rem;
    line-height: 1.35;
  }

  .recall-box {
    padding: 1rem 1.1rem;
    border-radius: 1.2rem;
    background: linear-gradient(135deg, color-mix(in srgb, var(--accent) 18%, white 82%), color-mix(in srgb, var(--panel-alt) 74%, white 26%));
    margin-bottom: 1rem;
  }

  .recall-box p {
    margin: 0 0 0.65rem;
    font-family: var(--font-ui);
    font-weight: 700;
  }

  .recall-box ul {
    margin: 0;
    padding-left: 1.2rem;
    color: var(--muted);
  }

  .front-actions {
    padding: 0.2rem 0 1rem;
  }

  .primary {
    border: 0;
    border-radius: 999px;
    background: var(--accent);
    color: #fff;
    padding: 0.9rem 1.2rem;
    font-family: var(--font-ui);
    font-weight: 700;
  }

  .answer {
    margin-bottom: 1.2rem;
  }

  .insight-box,
  .example-box,
  .quiz-box {
    margin-bottom: 1rem;
    padding: 0.95rem 1rem;
    border-radius: 1rem;
    border: 1px solid var(--border);
    background: color-mix(in srgb, var(--panel-alt) 82%, white 18%);
  }

  h3 {
    margin: 0 0 0.6rem;
    font-size: 1rem;
    font-family: var(--font-ui);
  }

  .insight-box ul {
    margin: 0;
    padding-left: 1.1rem;
  }

  .insight-box li + li {
    margin-top: 0.35rem;
  }

  .example-head {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 1rem;
  }

  .de-example {
    margin: 0;
    font-family: var(--font-ui);
    font-weight: 700;
  }

  .zh-example {
    margin: 0.35rem 0 0;
    color: var(--muted);
  }

  .ghost {
    border: 1px solid var(--border);
    border-radius: 999px;
    background: transparent;
    padding: 0.32rem 0.72rem;
    font-family: var(--font-ui);
    font-size: 0.85rem;
  }

  .quiz-box p {
    margin: 0 0 0.65rem;
  }

  .quiz-options {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 0.6rem;
  }

  .quiz-options button {
    border: 1px solid var(--border);
    background: #fff;
    border-radius: 0.8rem;
    padding: 0.62rem 0.7rem;
    text-align: left;
    font-family: var(--font-ui);
  }

  .quiz-options button.selected {
    border-color: var(--accent);
    background: color-mix(in srgb, var(--accent) 16%, white 84%);
  }

  .quiz-feedback {
    margin-top: 0.7rem;
    margin-bottom: 0;
    font-size: 0.92rem;
  }

  .quiz-feedback.ok {
    color: #0d725f;
  }

  .quiz-feedback.bad {
    color: var(--danger);
  }

  @media (max-width: 720px) {
    .stats-grid {
      grid-template-columns: 1fr;
    }

    .meta {
      align-items: flex-start;
      flex-direction: column;
    }

    .quiz-options {
      grid-template-columns: 1fr;
    }
  }
</style>
