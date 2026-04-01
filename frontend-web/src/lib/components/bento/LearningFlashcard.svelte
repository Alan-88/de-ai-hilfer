<script lang="ts">
  import { parseAnalysisMarkdown } from "$lib/analysis/structuredAnalysis";
  import { buildLearningEnhancements } from "$lib/learningEnhance";
  import type { LearningSessionWord, ReviewQuality } from "$lib/types";

  let { wordData, isSubmitting = false, onRate } = $props<{
    wordData: LearningSessionWord;
    isSubmitting: boolean;
    onRate: (q: ReviewQuality) => void;
  }>();

  const ratingOptions: Array<{ label: string; value: ReviewQuality; className: string }> = [
    { label: "完全忘记", value: 0 as ReviewQuality, className: "rate-btn hard" },
    { label: "提示后记起", value: 2 as ReviewQuality, className: "rate-btn" },
    { label: "犹豫但正确", value: 4 as ReviewQuality, className: "rate-btn" },
    { label: "完美回忆", value: 5 as ReviewQuality, className: "rate-btn easy" },
  ];

  function learningStateLabel(state?: number | null) {
    switch (state) {
      case 1: return "学习中";
      case 2: return "复习期";
      case 3: return "重学";
      default: return "新词";
    }
  }

  function formatDate(value?: string | null) {
    if (!value) return "首次复习";
    return new Intl.DateTimeFormat("zh-CN", {
      month: "numeric", day: "numeric",
      hour: "2-digit", minute: "2-digit",
    }).format(new Date(value));
  }

  const structured = $derived(parseAnalysisMarkdown(wordData.analysis_markdown, wordData.query_text));
  const enhancement = $derived(buildLearningEnhancements(wordData.analysis_markdown, wordData.query_text));
  const primaryMeaning = $derived(structured.meanings[0]);
  const primaryExample = $derived(structured.examples[0] ?? enhancement.examples[0] ?? null);
  
  let showingAnswer = $state(false);

  // 当 wordData 改变时重置显示状态
  $effect(() => {
    if (wordData) showingAnswer = false;
  });
</script>

<div class="flashcard-shell" class:revealed={showingAnswer}>
  <div class="bento-card flashcard">
    <div class="flashcard-meta">
      <span class="pill">{learningStateLabel(wordData.progress?.state)} · {wordData.repetitions_left} 次待记</span>
      <span class="pill">上次: {formatDate(wordData.progress?.last_reviewed_at)}</span>
    </div>

    <div class="flashcard-front">
      <h2>{wordData.query_text}</h2>
      {#if !showingAnswer}
        <button class="btn-primary show-btn" onclick={() => showingAnswer = true}>
          <i class="ph ph-eye"></i> 查看解析
        </button>
      {/if}
    </div>

    {#if showingAnswer}
      <div class="flashcard-back">
        <div class="meanings-row">
          {#if primaryMeaning}
            <span class="pos-tag">{primaryMeaning.partOfSpeech}</span>
            <span class="zh-main">{primaryMeaning.chinese}</span>
          {/if}
          {#if structured.grammarRows[0]}
             <span class="grammar-pill">{structured.grammarRows[0].value}</span>
          {/if}
        </div>

        <div class="detail-section">
          {#if primaryExample}
            <div class="example-box">
              <div class="de-text">{primaryExample.de}</div>
              {#if primaryExample.zh}
                <div class="zh-text">{primaryExample.zh}</div>
              {/if}
            </div>
          {/if}

          {#if enhancement.quiz}
            <div class="quiz-box">
              <p class="quiz-q"><strong>测验：</strong>{enhancement.quiz.question}</p>
              <div class="quiz-options">
                {#each enhancement.quiz.options as option}
                  <span class="opt-pill">{option}</span>
                {/each}
              </div>
            </div>
          {/if}
        </div>
      </div>
    {/if}
  </div>

  {#if showingAnswer}
    <div class="rating-bar">
      {#each ratingOptions as option}
        <button 
          class={option.className} 
          disabled={isSubmitting} 
          onclick={() => onRate(option.value)}
        >
          {option.label}
        </button>
      {/each}
    </div>
  {/if}
</div>

<style>
  .flashcard-shell { width: 100%; display: flex; flex-direction: column; gap: 1.5rem; }
  
  .flashcard {
    min-height: 420px;
    display: flex;
    flex-direction: column;
    position: relative;
    padding: 2.5rem;
    background: var(--card-bg);
    transition: transform 0.4s cubic-bezier(0.2, 0.8, 0.2, 1);
  }

  .flashcard-meta {
    display: flex;
    justify-content: space-between;
    width: 100%;
    margin-bottom: 2rem;
  }
  .pill { font-size: 0.8rem; font-weight: 700; color: var(--text-muted); background: var(--btn-secondary); padding: 0.3rem 0.6rem; border-radius: var(--radius-sm); }

  .flashcard-front {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    text-align: center;
  }

  .flashcard-front h2 { font-size: 3.5rem; font-weight: 800; margin-bottom: 2rem; letter-spacing: -0.02em; }
  
  .show-btn { padding: 1rem 2.5rem; font-size: 1.1rem; box-shadow: var(--shadow-sm); }

  .flashcard-back {
    animation: slideUpFade 0.4s ease forwards;
    width: 100%;
  }

  @keyframes slideUpFade {
    from { opacity: 0; transform: translateY(15px); }
    to { opacity: 1; transform: translateY(0); }
  }

  .meanings-row { display: flex; align-items: center; gap: 0.75rem; flex-wrap: wrap; margin-bottom: 1.5rem; border-bottom: 1px solid var(--border-color); padding-bottom: 1.5rem; }
  .pos-tag { background: var(--accent-main); color: white; padding: 0.2rem 0.6rem; border-radius: 6px; font-weight: 800; font-size: 0.9rem; }
  .zh-main { font-size: 1.5rem; font-weight: 700; color: var(--text-main); }
  .grammar-pill { color: var(--accent-main); font-weight: 700; font-family: var(--font-serif); }

  .detail-section { display: flex; flex-direction: column; gap: 1.25rem; }
  
  .example-box { background: var(--bg-color); padding: 1.25rem; border-radius: var(--radius-md); }
  .de-text { font-size: 1.15rem; font-weight: 600; line-height: 1.5; color: var(--text-main); }
  .zh-text { font-size: 0.95rem; color: var(--text-muted); margin-top: 0.5rem; }

  .quiz-box { border: 1px dashed var(--border-color); padding: 1.15rem; border-radius: var(--radius-md); }
  .quiz-q { margin-bottom: 0.75rem; font-size: 0.95rem; }
  .quiz-options { display: flex; flex-wrap: wrap; gap: 0.5rem; }
  .opt-pill { font-size: 0.85rem; padding: 0.25rem 0.5rem; border: 1px solid var(--border-color); border-radius: 6px; color: var(--text-muted); }

  .rating-bar {
    display: grid;
    grid-template-columns: repeat(4, 1fr);
    gap: 0.75rem;
    animation: fadeIn 0.5s ease;
  }

  .rate-btn {
    padding: 1.1rem 0.5rem;
    border-radius: var(--radius-md);
    font-weight: 700;
    font-size: 0.95rem;
    background: var(--btn-secondary);
    color: var(--btn-secondary-text);
    box-shadow: var(--shadow-sm);
  }

  .rate-btn:hover { transform: translateY(-3px); box-shadow: var(--shadow-hover); }
  .rate-btn.easy { background: var(--success-bg); color: var(--success-text); }
  .rate-btn.hard { background: var(--danger-bg); color: var(--danger-text); }

  @media (max-width: 600px) {
    .rating-bar { grid-template-columns: repeat(2, 1fr); }
    .flashcard-front h2 { font-size: 2.5rem; }
  }
</style>
