<script lang="ts">
  type ArticleItem = {
    id: number;
    title: string;
    path: string;
    memory: number | null;
    marks: number;
    lastOpened: string;
  };
  type DragItem = { kind: "folder" | "article"; id: number };

  interface Props {
    articles: ArticleItem[];
    selectedId: number | null;
    onSelect: (id: number) => void;
    onDragStart: (event: DragEvent, item: DragItem) => void;
    onDragEnd: () => void;
    onContextMenu: (event: MouseEvent, kind: "article", id: number) => void;
    onPressStart: (event: PointerEvent, kind: "article", id: number) => void;
    onPressMove: (event: PointerEvent) => void;
    onPressEnd: () => void;
  }

  let {
    articles, selectedId, onSelect, onDragStart, onDragEnd, onContextMenu,
    onPressStart, onPressMove, onPressEnd,
  }: Props = $props();
</script>

<div class="article-list" aria-label="文章目录">
  {#each articles as article (article.id)}
    <div
      class:current={selectedId === article.id}
      class="article-row"
      role="button"
      tabindex="0"
      onclick={() => onSelect(article.id)}
      onkeydown={(event) => { if (event.key === "Enter" || event.key === " ") onSelect(article.id); }}
      oncontextmenu={(event) => onContextMenu(event, "article", article.id)}
      onpointerdown={(event) => onPressStart(event, "article", article.id)}
      onpointermove={onPressMove}
      onpointerup={onPressEnd}
      onpointercancel={onPressEnd}
    >
      <button
        class="drag-handle"
        draggable="true"
        aria-label={`拖动文章 ${article.title}`}
        title="拖动文章"
        onclick={(event) => event.stopPropagation()}
        ondragstart={(event) => onDragStart(event, { kind: "article", id: article.id })}
        ondragend={onDragEnd}
      ><i class="ph ph-file-text"></i></button>
      <span class="title"><strong>{article.title}</strong><small>{article.path}</small></span>
      <span class="memory" title={article.memory === null ? "暂无标记词" : `当前记忆度 ${article.memory}%`}>
        {#if article.memory === null}
          <em>—</em>
        {:else}
          <i><b style:width={`${article.memory}%`}></b></i><em>{article.memory}%</em>
        {/if}
      </span>
      <span class="marks" title={`${article.marks} 个标记词`}><i class="ph ph-bookmark-simple"></i>{article.marks}</span>
      <span class="date" title="上次阅读">{article.lastOpened}</span>
    </div>
  {:else}
    <div class="empty">这个位置还没有文章</div>
  {/each}
</div>

<style>
  .article-list { border-top: 1px solid var(--border-color); }
  .article-row { display: grid; width: 100%; min-height: 68px; grid-template-columns: 26px minmax(220px, 1.7fr) 150px 58px 76px; align-items: center; gap: 0.75rem; padding: 0 0.65rem; border-bottom: 1px solid var(--border-color); color: var(--text-main); text-align: left; transition: background 120ms ease; }
  .article-row:hover, .article-row.current { background: color-mix(in srgb, var(--accent-main) 6%, transparent); }
  .article-row:focus-visible { outline: 2px solid color-mix(in srgb, var(--accent-main) 55%, transparent); outline-offset: -2px; }
  .drag-handle { display: grid; width: 26px; height: 32px; place-items: center; border-radius: 7px; background: transparent; color: var(--accent-main); cursor: grab; }
  .drag-handle:hover { background: var(--btn-secondary); }
  .drag-handle:active { cursor: grabbing; }
  .title { display: grid; min-width: 0; gap: 0.2rem; }
  .title strong, .title small { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .title strong { font-size: 0.84rem; cursor: text; user-select: text; }
  .title small, .date { color: var(--text-muted); font-size: 0.7rem; }
  .memory { display: flex; align-items: center; gap: 0.5rem; }
  .memory > i { width: 94px; height: 4px; overflow: hidden; border-radius: 2px; background: var(--btn-secondary); }
  .memory b { display: block; height: 100%; background: var(--accent-main); }
  .memory em, .marks { color: var(--text-muted); font-size: 0.68rem; font-style: normal; }
  .marks { display: flex; align-items: center; gap: 0.25rem; }
  .empty { display: grid; min-height: 180px; place-items: center; color: var(--text-muted); font-size: 0.78rem; }
  @media (max-width: 650px) {
    .article-row { grid-template-columns: 26px minmax(0, 1fr) 50px; gap: 0.55rem; }
    .memory, .date { display: none; }
    .title strong { font-size: 0.82rem; }
  }
  @media (pointer: coarse) { .title strong { cursor: default; user-select: none; } }
</style>
