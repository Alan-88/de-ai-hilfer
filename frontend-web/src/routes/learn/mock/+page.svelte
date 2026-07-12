<script lang="ts">
  import LearningFlashcard from "$lib/components/bento/LearningFlashcard.svelte";
  import type {
    LearningRecallRating,
    LearningSessionWord,
    LearningStatsResponse,
  } from "$lib/types";

  const mockWords = [
    {
      entry_id: 900001,
      query_text: "vertragen",
      analysis_markdown: `#### 应用与例句 (Anwendung & Beispiele)

etwas (Akk.) vertragen
用于描述身体对食物、药物、酒精或环境的耐受能力。支配第四格 (Akkusativ)。
1. Ich vertrage keine Milchprodukte.
- 我不能吃乳制品。

sich mit jemandem vertragen
描述人际关系中的相处融洽，或争执之后重新和好。
1. Nach dem Streit haben sie sich wieder vertragen.
- 争吵之后他们又和好了。

#### 语法详情 (Grammatik)
| 特征 | 详情 |
| :--- | :--- |
| 词性 | starkes Verb |
| 支配 | Akkusativ / reflexiv mit Dativ |
| 关键形式 | verträgt, vertrug, hat vertragen |
`,
      structured_analysis: {
        headword: "vertragen",
        phonetic: "",
        meanings: [
          { part_of_speech: "v.", chinese: "忍受；耐受；相处", english: "to tolerate; to get along" },
        ],
        usage_modules: [
          {
            title: "etwas vertragen",
            explanation: "用于身体或心理层面的耐受。",
            example_de: "Ich vertrage keinen Lärm.",
            example_zh: "我受不了噪音。",
          },
        ],
        collocations: ["etwas vertragen", "sich mit jemandem vertragen"],
        examples: [
          { de: "Ich vertrage keine Milchprodukte.", zh: "我不能吃乳制品。" },
        ],
        grammar_rows: [
          { key: "Verbtyp", value: "stark · reflexiv möglich" },
        ],
        grammar_branches: [
          {
            selector: "vt.",
            pos: "verb",
            meanings: [{ zh: "忍受；耐受", en: "to tolerate" }],
            grammar: {
              genders: [],
              noun_class: "",
              plural_forms: [],
              genitive_forms: [],
              separable: "no",
              transitivity: "transitive",
              reflexive: "optional",
              auxiliaries: ["haben"],
              present_3sg: "verträgt",
              preterite_3sg: "vertrug",
              partizip_ii: "vertragen",
              comparative: "",
              superlative: "",
              governs_cases: ["accusative"],
              word_order: "",
            },
          },
        ],
        word_network: null,
        deep_insights: [],
      },
      repetitions_left: 5,
      progress: {
        entry_id: 900001,
        review_count: 0,
        next_review_at: new Date().toISOString(),
        scheduled_days: 0,
        stability: 0,
        difficulty: 0,
        state: 0,
        lapses: 0,
      },
      phase: "new",
      appearance_count_today: 1,
    },
    {
      entry_id: 900002,
      query_text: "während",
      analysis_markdown: `#### 应用与例句 (Anwendung & Beispiele)

während + Genitiv
表示“在……期间”，作为介词时后接第二格 (Genitiv)。
1. Während des Unterrichts bleibt das Handy leise.
- 上课期间手机保持静音。

während als Konjunktion
作为连词时引导从句，变位动词放在句末。
1. Ich koche, während du den Tisch deckst.
- 我做饭的时候，你摆餐具。
`,
      structured_analysis: {
        headword: "während",
        phonetic: "",
        meanings: [
          { part_of_speech: "prep.", chinese: "在……期间", english: "during" },
          { part_of_speech: "conj.", chinese: "当……时候；而", english: "while; whereas" },
        ],
        usage_modules: [
          {
            title: "während + Genitiv",
            explanation: "介词用法，标记一段时间范围。",
            example_de: "Während des Urlaubs lese ich viel.",
            example_zh: "假期期间我读很多书。",
          },
        ],
        collocations: ["während des Unterrichts", "während der Woche"],
        examples: [
          { de: "Ich lese, während er schläft.", zh: "他睡觉的时候我在读书。" },
        ],
        grammar_rows: [
          { key: "支配", value: "Genitiv" },
          { key: "语序", value: "Nebensatz: Verb am Ende" },
        ],
        grammar_branches: [],
        word_network: null,
        deep_insights: [],
      },
      repetitions_left: 4,
      progress: {
        entry_id: 900002,
        review_count: 3,
        next_review_at: new Date().toISOString(),
        last_reviewed_at: "2026-07-03T10:20:00.000Z",
        scheduled_days: 2,
        stability: 2.7,
        difficulty: 6.1,
        state: 2,
        lapses: 1,
      },
      phase: "review",
      appearance_count_today: 1,
    },
    {
      entry_id: 900003,
      query_text: "Erinnerung",
      analysis_markdown: `#### 应用与例句 (Anwendung & Beispiele)

die Erinnerung an jemanden/etwas
表示“对某人或某事的记忆、纪念”。注意 an 后常接第四格 (Akkusativ)。
1. Dieses Foto ist meine einzige Erinnerung an meine Großmutter.
- 这张照片是我对祖母唯一的纪念。

#### 语法详情 (Grammatik)
| 特征 | 详情 |
| :--- | :--- |
| 词性 | die Erinnerung |
| 复数 | die Erinnerungen |
`,
      structured_analysis: {
        headword: "Erinnerung",
        phonetic: "",
        meanings: [
          { part_of_speech: "n.", chinese: "记忆；回忆；纪念", english: "memory; remembrance" },
        ],
        usage_modules: [
          {
            title: "die Erinnerung an ...",
            explanation: "用于表达对某人或某事的回忆或纪念。",
            example_de: "Das ist eine schöne Erinnerung an unsere Reise.",
            example_zh: "这是我们旅行的一段美好回忆。",
          },
        ],
        collocations: ["in Erinnerung bleiben", "die Erinnerung an etwas"],
        examples: [
          { de: "Der Tag bleibt mir in guter Erinnerung.", zh: "那一天给我留下了美好的回忆。" },
        ],
        grammar_rows: [
          { key: "Genus", value: "die" },
          { key: "Plural", value: "Erinnerungen" },
        ],
        grammar_branches: [],
        word_network: null,
        deep_insights: [],
      },
      repetitions_left: 3,
      progress: {
        entry_id: 900003,
        review_count: 1,
        next_review_at: new Date().toISOString(),
        last_reviewed_at: "2026-07-09T07:10:00.000Z",
        scheduled_days: 1,
        stability: 1.3,
        difficulty: 7.2,
        state: 1,
        lapses: 0,
      },
      phase: "intraday",
      appearance_count_today: 3,
    },
  ] satisfies LearningSessionWord[];

  const stats: LearningStatsResponse = {
    total_words: 4,
    due_today: 4,
    average_stability: 1.34,
  };

  let activeIndex = $state(0);
  let completedCount = $state(1);
  let intradayQueueCount = $state(2);
  let lastRating = $state<LearningRecallRating | null>(null);

  const currentWord = $derived(mockWords[activeIndex]);
  const sessionProgress = $derived((completedCount / mockWords.length) * 100);

  function chooseWord(index: number) {
    activeIndex = index;
    lastRating = null;
  }

  function handleReviewed(rating: LearningRecallRating) {
    lastRating = rating;
    if (rating === "known") {
      completedCount = Math.min(completedCount + 1, mockWords.length);
      intradayQueueCount = Math.max(intradayQueueCount - 1, 0);
    } else {
      intradayQueueCount += 1;
    }
    activeIndex = (activeIndex + 1) % mockWords.length;
  }
</script>

<svelte:head>
  <title>学习页 Mock | De-AI-Hilfer</title>
</svelte:head>

<header class="page-header">
  <div>
    <p class="mock-label">Mock only · 不连接后端</p>
    <h1>沉浸复习</h1>
    <p>这里复用真实学习卡片组件，但所有数据和评分都只停留在浏览器本地。</p>
  </div>
</header>

<div class="stats-grid">
  <div class="bento-card stat-card">
    <div class="label">当前轮次</div>
    <div class="num">{completedCount}/{mockWords.length}</div>
  </div>
  <div class="bento-card stat-card">
    <div class="label">学习中</div>
    <div class="num">{stats.total_words}</div>
  </div>
  <div class="bento-card stat-card">
    <div class="label">今日待复习</div>
    <div class="num">{stats.due_today}</div>
  </div>
  <div class="bento-card stat-card">
    <div class="label">日内队列</div>
    <div class="num">{intradayQueueCount}</div>
  </div>
</div>

<div class="mock-switcher" aria-label="选择 mock 场景">
  {#each mockWords as word, index}
    <button
      class:active={activeIndex === index}
      type="button"
      onclick={() => chooseWord(index)}
    >
      <span>{word.phase === "new" ? "新词" : word.phase === "review" ? "复习" : "今日重现"}</span>
      {word.query_text}
    </button>
  {/each}
</div>

{#if lastRating}
  <div class="mock-toast">
    已模拟点击：{lastRating === "known" ? "认识" : lastRating === "fuzzy" ? "模糊" : "忘记/不认识"}
  </div>
{/if}

<div class="flashcard-container">
  <div class="progress-bar-wrapper">
    <div class="progress-fill" style="width: {sessionProgress}%"></div>
  </div>

  <LearningFlashcard
    wordData={currentWord}
    isSubmitting={false}
    onRate={handleReviewed}
  />
</div>

<style>
  .page-header {
    width: 100%;
    margin-bottom: 2rem;
  }

  .mock-label {
    color: var(--accent-main);
    font-weight: 800;
    font-size: 0.78rem;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    margin-bottom: 0.5rem;
  }

  .page-header h1 {
    font-size: 2.25rem;
    font-weight: 800;
    margin-bottom: 0.5rem;
  }

  .page-header p:not(.mock-label) {
    color: var(--text-muted);
    font-size: 1.05rem;
  }

  .stats-grid {
    display: grid;
    grid-template-columns: repeat(4, 1fr);
    gap: 1.25rem;
    width: 100%;
    margin-bottom: 1.25rem;
  }

  .stat-card {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding: 1.5rem;
    text-align: center;
  }

  .stat-card .label {
    font-size: 0.85rem;
    font-weight: 700;
    color: var(--text-muted);
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .stat-card .num {
    font-size: 2rem;
    font-weight: 800;
    color: var(--accent-main);
    margin-top: 0.5rem;
  }

  .mock-switcher {
    width: 100%;
    display: flex;
    gap: 0.75rem;
    flex-wrap: wrap;
    margin-bottom: 1rem;
  }

  .mock-switcher button {
    display: inline-flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.7rem 0.9rem;
    border-radius: var(--radius-md);
    background: var(--btn-secondary);
    color: var(--btn-secondary-text);
    box-shadow: var(--shadow-sm);
    font-weight: 800;
  }

  .mock-switcher button.active {
    background: var(--accent-main);
    color: white;
  }

  .mock-switcher span {
    font-size: 0.75rem;
    opacity: 0.75;
  }

  .mock-toast {
    width: 100%;
    padding: 0.85rem 1rem;
    margin-bottom: 1rem;
    border-radius: var(--radius-md);
    background: var(--success-bg);
    color: var(--success-text);
    font-weight: 700;
  }

  .flashcard-container {
    width: 100%;
    max-width: 640px;
    margin: 0 auto;
  }

  .progress-bar-wrapper {
    width: 100%;
    height: 8px;
    background: var(--btn-secondary);
    border-radius: var(--radius-full);
    margin-bottom: 2rem;
    overflow: hidden;
  }

  .progress-fill {
    height: 100%;
    background: var(--accent-main);
    transition: width 0.5s cubic-bezier(0.4, 0, 0.2, 1);
  }

  @media (max-width: 768px) {
    .stats-grid {
      grid-template-columns: repeat(2, 1fr);
    }
  }
</style>
