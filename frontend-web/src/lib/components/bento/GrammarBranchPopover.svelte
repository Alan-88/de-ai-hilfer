<script lang="ts">
  import type { GrammarBranch } from "$lib/types";
  import { fade, scale } from "svelte/transition";
  import { backOut } from "svelte/easing";
  import { onMount, tick } from "svelte";

  let {
    branch,
    triggerRect,
    onClose
  }: {
    branch: GrammarBranch;
    triggerRect: DOMRect | null;
    onClose: () => void;
  } = $props();

  // 定义词性分组展示字段及固定顺序
  const posSchemaRules: Record<string, string[]> = {
    verb: ["present_3sg", "preterite_3sg", "partizip_ii", "auxiliaries", "governs_cases"],
    noun: ["plural_forms", "genitive_forms", "noun_class"],
    adjective: ["comparative", "superlative"],
    adverb: ["comparative", "superlative"],
    preposition: ["governs_cases"],
    pronoun: ["plural_forms", "genitive_forms", "noun_class"],
    article: ["plural_forms", "genitive_forms", "noun_class"],
    conjunction: ["word_order"]
  };

  const labelMap: Record<string, string> = {
    noun_class: "名词类别",
    plural_forms: "复数形式",
    genitive_forms: "第二格",
    auxiliaries: "助动词",
    present_3sg: "现在时(单三)",
    preterite_3sg: "过去时(单三)",
    partizip_ii: "过去分词",
    comparative: "比较级",
    superlative: "最高级",
    governs_cases: "支配格",
    word_order: "句法站位"
  };

  // 值翻译映射
  const caseMap: Record<string, string> = {
    "nominative": "Nominativ", "genitive": "Genitiv", "dative": "Dativ", "accusative": "Akkusativ",
    "nominativ": "Nominativ", "genitiv": "Genitiv", "dativ": "Dativ", "akkusativ": "Akkusativ"
  };
  const nounClassMap: Record<string, string> = {
    "strong": "stark", "weak": "schwach", "mixed": "gemischt"
  };
  const wordOrderMap: Record<string, string> = {
    "main_clause": "Hauptsatz (正语序)", "subordinate_clause": "Nebensatz (尾语序)"
  };

  // 整理数据：基于 Schema 严格获取对应词性的字段
  const displayFields = $derived(() => {
    const pos = (branch.pos || "").toLowerCase();
    const posTokens = pos.split(/[\s,]+/);

    // 匹配当前词性的 Schema，确保词性单词精准对应
    const schemaKeys = Object.entries(posSchemaRules).find(([key]) => posTokens.includes(key))?.[1];

    if (!schemaKeys) return []; // 对于不包含语法特性的词性，返回空

    return schemaKeys.map(key => {
      const value = branch.grammar[key as keyof typeof branch.grammar];
      const label = labelMap[key] || key;
      let displayValue = "";

      const isEmpty = (v: any) => v === null || v === undefined || v === "" || v === "none" || v === "unknown" || (Array.isArray(v) && v.length === 0);

      if (isEmpty(value)) {
        displayValue = "-";
      } else {
        // 根据不同 Key 进行专属格式化或翻译
        if (key === "auxiliaries" && Array.isArray(value)) {
          displayValue = value.join("/");
        }
        else if (key === "governs_cases") {
          const vals = Array.isArray(value) ? value : [value.toString()];
          displayValue = vals.map(v => caseMap[v.toLowerCase().trim()] || (v.charAt(0).toUpperCase() + v.slice(1))).join(", ");
        }
        else if (key === "noun_class") {
          const v = value.toString().toLowerCase().trim();
          displayValue = nounClassMap[v] || value.toString();
        }
        else if (key === "word_order") {
          const v = value.toString().toLowerCase().trim();
          displayValue = wordOrderMap[v] || value.toString();
        }
        else if (Array.isArray(value)) {
          displayValue = value.join(", ");
        }
        else {
          displayValue = value.toString().trim();
        }
      }

      return { key, label, value: displayValue };
    });
  });

  function formatBranchMeanings(meanings: {zh: string, en: string}[]): string {
    return meanings.map(m => m.zh).filter(Boolean).join("；");
  }

  // 气泡定位逻辑：使用 absolute 定位使其跟随页面滚动
  let popoverStyle = $state("visibility: hidden;");
  let arrowStyle = $state("");
  let transformOrigin = $state("center top");
  let popoverRef: HTMLElement;

  async function updatePosition() {
    if (!triggerRect) return;
    await tick();

    const padding = 12;
    const cardWidth = 320;
    const viewportWidth = window.innerWidth;

    // 计算基于 Viewport 的 left
    let left = triggerRect.left + (triggerRect.width / 2) - (cardWidth / 2);
    if (left + cardWidth > viewportWidth - padding) left = viewportWidth - cardWidth - padding;
    if (left < padding) left = padding;

    // 获取定位父级（.search-page-container 或 body）
    const container = document.getElementById("page-search") || document.body;
    const containerRect = container.getBoundingClientRect();

    // 转换为基于定位父级的 absolute 坐标
    let absoluteLeft = left - containerRect.left;

    let top = triggerRect.bottom + 8;
    let placement: 'bottom' | 'top' = 'bottom';

    // 预估高度进行边界检查
    if (top + 300 > window.innerHeight && triggerRect.top > 300) {
      placement = 'top';
    }

    // 使用 absolute 定位
    if (placement === 'bottom') {
      let absoluteTop = triggerRect.bottom - containerRect.top + 8;
      popoverStyle = `left: ${absoluteLeft}px; top: ${absoluteTop}px; width: ${cardWidth}px; visibility: visible;`;
    } else {
      // 位于上方时
      let absoluteBottom = containerRect.height - (triggerRect.top - containerRect.top) + 8;
      popoverStyle = `left: ${absoluteLeft}px; bottom: ${absoluteBottom}px; width: ${cardWidth}px; visibility: visible;`;
    }

    // 箭头位置（基于气泡内部坐标）
    const arrowLeft = triggerRect.left + (triggerRect.width / 2) - left;
    transformOrigin = `${arrowLeft}px ${placement === 'bottom' ? '0' : '100%'}`;

    if (placement === 'bottom') {
      arrowStyle = `left: ${arrowLeft}px; top: -6px; border-top: 1px solid var(--border-color); border-left: 1px solid var(--border-color);`;
    } else {
      arrowStyle = `left: ${arrowLeft}px; bottom: -6px; border-bottom: 1px solid var(--border-color); border-right: 1px solid var(--border-color);`;
    }
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === "Escape") onClose();
  }

  function handleWindowClick(e: MouseEvent) {
    // 如果点击的是气泡外部，则关闭
    if (popoverRef && !popoverRef.contains(e.target as Node)) {
      onClose();
    }
  }

  onMount(() => {
    updatePosition();
    window.addEventListener("keydown", handleKeydown);
    window.addEventListener("resize", onClose, { once: true });

    // 延迟绑定点击事件，防止触发按钮自身的点击事件立即将其关闭
    setTimeout(() => {
      window.addEventListener("click", handleWindowClick);
    }, 0);

    return () => {
      window.removeEventListener("keydown", handleKeydown);
      window.removeEventListener("resize", onClose);
      window.removeEventListener("click", handleWindowClick);
    };
  });
</script>

<!-- svelte-ignore a11y_click_events_have_key_events -->
<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
  class="popover-card"
  bind:this={popoverRef}
  style="{popoverStyle} transform-origin: {transformOrigin};"
  onclick={(e) => e.stopPropagation()}
  transition:scale={{ start: 0.85, duration: 350, easing: backOut }}
>
  <div class="popover-arrow" style={arrowStyle}></div>

  <header class="popover-header">
    <div class="header-main">
      <span class="branch-selector">{branch.selector}</span>
      <h3 class="branch-title">{formatBranchMeanings(branch.meanings) || "语法详情"}</h3>
    </div>
  </header>

  <div class="popover-body">
    {#if displayFields().length > 0}
      <div class="field-grid">
        {#each displayFields() as field}
          <div class="field-item">
            <span class="field-label">{field.label}</span>
            <span class="field-value">{field.value}</span>
          </div>
        {/each}
      </div>
    {:else}
      <p class="empty-msg">无特殊语法特征</p>
    {/if}
  </div>
</div>

<style>
  .popover-card {
    position: absolute;
    background: color-mix(in srgb, var(--card-bg) 70%, transparent);
    border: 1px solid var(--border-color);
    border-radius: var(--radius-lg);
    /* 多层精细阴影：环境光 + 聚光 + 顶部高光 */
    box-shadow:
      0 4px 12px rgba(0, 0, 0, 0.05),
      0 20px 50px -12px rgba(0, 0, 0, 0.15),
      inset 0 1px 1px rgba(255, 255, 255, 0.4);
    display: flex;
    flex-direction: column;
    overflow: visible;
    z-index: 3001;
    backdrop-filter: blur(24px) saturate(180%);
    -webkit-backdrop-filter: blur(24px) saturate(180%);
  }

  .popover-arrow {
    position: absolute;
    width: 14px;
    height: 14px;
    background: color-mix(in srgb, var(--card-bg) 75%, transparent);
    transform: translateX(-50%) rotate(45deg);
    border-radius: 4px 0 0 0; /* 尖端微圆润 */
    z-index: -1;
  }

  .popover-header {
    padding: 0.9rem 1.5rem 0.4rem 1.5rem; /* 显著减少垂直留白 */
    /* 微妙的顶部渐变增强材质感 */
    background: linear-gradient(to bottom, color-mix(in srgb, var(--bg-color) 40%, transparent), transparent);
    border-bottom: 1px solid rgba(0,0,0,0.04);
    border-radius: var(--radius-lg) var(--radius-lg) 0 0;
  }

  .header-main {
    display: flex;
    flex-direction: column;
    gap: 0.15rem;
  }

  .branch-selector {
    font-size: 0.65rem;
    font-weight: 900;
    text-transform: uppercase;
    color: var(--accent-main);
    letter-spacing: 0.08em;
    opacity: 0.8;
  }

  .branch-title {
    font-size: 1.15rem;
    font-weight: 800;
    margin: 0;
    color: var(--text-main);
    letter-spacing: -0.01em;
  }

  .popover-body {
    padding: 0.4rem 1.5rem 1.25rem 1.5rem; /* 显著减少顶部垂直留白 */
    max-height: 45vh;
    overflow-y: auto;
  }

  .field-grid {
    display: flex;
    flex-direction: column;
    gap: 0;
  }

  .field-item {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 1.5rem;
    padding: 0.75rem 0;
    /* 替换为更有呼吸感的虚线 */
    border-bottom: 1px dashed rgba(0,0,0,0.06);
  }

  .field-item:last-child {
    border-bottom: none;
  }

  .field-label {
    font-size: 0.78rem;
    color: var(--text-muted);
    font-weight: 600;
    flex-shrink: 0;
    opacity: 0.75;
  }

  .field-value {
    font-size: 1.05rem;
    font-weight: 800;
    color: var(--text-main);
    text-align: right;
    line-height: 1.3;
    letter-spacing: -0.01em;
  }

  .empty-msg {
    text-align: center;
    color: var(--text-muted);
    padding: 1.5rem 0;
    font-style: italic;
    font-size: 0.85rem;
  }

  :global(.canvas-dark) .popover-card {
    background: color-mix(in srgb, var(--card-bg) 65%, transparent);
    border-color: rgba(255, 255, 255, 0.1);
    box-shadow:
      0 4px 12px rgba(0, 0, 0, 0.2),
      0 20px 50px -12px rgba(0, 0, 0, 0.4),
      inset 0 1px 1px rgba(255, 255, 255, 0.05);
  }

  :global(.canvas-dark) .popover-arrow {
    background: color-mix(in srgb, var(--card-bg) 70%, transparent);
    border-color: rgba(255, 255, 255, 0.1);
  }

  :global(.canvas-dark) .field-item {
    border-bottom-color: rgba(255, 255, 255, 0.06);
  }

  :global(.canvas-dark) .popover-header {
    background: linear-gradient(to bottom, rgba(255,255,255,0.03), transparent);
    border-bottom-color: rgba(255,255,255,0.03);
  }

  :global(.canvas-dark) .field-value {
    color: #93c5fd;
  }

  @media (max-width: 480px) {
    .popover-card {
      position: fixed !important; /* 移动端保持 fixed 居中更稳健，或者跟随 absolute */
      width: calc(100vw - 24px) !important;
      left: 12px !important;
    }
    .popover-arrow {
      display: none;
    }
  }
</style>
