<script lang="ts">
  import ReadingBookmarkPicker from "$lib/components/reading/ReadingBookmarkPicker.svelte";
  import ReadingLookupRail from "$lib/components/reading/ReadingLookupRail.svelte";
  import { insertReadingBookmark, type ReadingTextTarget } from "$lib/readingBookmarks";
  import {
    getReadingArticle,
    getReadingIndex,
    markReadingArticleOpened,
    saveReadingArticle,
    type ReadingArticleDetail,
    type ReadingContentSegment,
  } from "$lib/readingApi";
  import { analyzeWord, getSuggestions } from "$lib/queryApi";
  import type { DBSuggestion } from "$lib/types";

  type MarkTarget = ReadingTextTarget & { x: number; y: number };

  let { articleId } = $props<{ articleId: number }>();

  let article = $state<ReadingArticleDetail | null>(null);
  let folderPath = $state("");
  let selectedKey = $state("");
  let selectedEntryId = $state<number | null>(null);
  let selectedOccurrence = $state<number | null>(null);
  let showBookmarks = $state(false);
  let railOpen = $state(false);
  let loading = $state(true);
  let error = $state("");
  let markTarget = $state<MarkTarget | null>(null);
  let markSuggestions = $state<DBSuggestion[]>([]);
  let markLoading = $state(false);
  let markSaving = $state(false);
  let markError = $state("");
  let markRequestId = 0;

  const bookmarks = $derived(
    article?.segments.filter((segment): segment is Extract<ReadingContentSegment, { kind: "bookmark" }> => segment.kind === "bookmark") ?? [],
  );

  $effect(() => { void loadArticle(articleId); });

  async function loadArticle(id: number) {
    loading = true;
    error = "";
    try {
      const [next, index] = await Promise.all([getReadingArticle(id), getReadingIndex()]);
      article = next;
      folderPath = buildFolderPath(next.folder_id, index.folders);
      const first = next.segments.find((segment) => segment.kind === "bookmark");
      if (first?.kind === "bookmark") selectWord(first, false);
      void markReadingArticleOpened(id).catch(() => {});
    } catch (cause) {
      article = null;
      error = cause instanceof Error ? cause.message : "文章加载失败";
    } finally {
      loading = false;
    }
  }

  function selectWord(bookmark: Extract<ReadingContentSegment, { kind: "bookmark" }>, revealRail = true) {
    selectedKey = bookmark.form_of;
    selectedEntryId = bookmark.entry_id;
    selectedOccurrence = bookmark.occurrence_index;
    if (revealRail) railOpen = true;
  }

  function jumpToBookmark(bookmark: Extract<ReadingContentSegment, { kind: "bookmark" }>) {
    selectWord(bookmark);
    showBookmarks = false;
    document.getElementById(`mark-${bookmark.occurrence_index}`)?.scrollIntoView({ behavior: "smooth", block: "center" });
  }

  function buildFolderPath(folderId: number | null, folders: Awaited<ReturnType<typeof getReadingIndex>>["folders"]) {
    if (folderId === null) return "未归类";
    const names: string[] = [];
    const seen = new Set<number>();
    let current = folders.find((folder) => folder.id === folderId);
    while (current && !seen.has(current.id)) {
      names.unshift(current.name);
      seen.add(current.id);
      current = current.parent_id === null ? undefined : folders.find((folder) => folder.id === current?.parent_id);
    }
    return names.join(" / ") || "未归类";
  }

  function wordCount() {
    if (!article) return 0;
    const text = article.segments.map((segment) => segment.kind === "text" ? segment.text : segment.surface).join("");
    return text.trim() ? text.trim().split(/\s+/).length : 0;
  }

  function handleArticleContextMenu(event: MouseEvent) {
    if (!article || (event.target instanceof Element && event.target.closest(".marked"))) return;
    const target = readingTextTarget(event);
    if (!target) return;
    event.preventDefault();
    markTarget = {
      ...target,
      x: Math.max(12, Math.min(event.clientX, window.innerWidth - 322)),
      y: Math.max(12, Math.min(event.clientY, window.innerHeight - 360)),
    };
    markSuggestions = [];
    markError = "";
    void loadMarkSuggestions(target.surface);
  }

  async function loadMarkSuggestions(surface: string) {
    const requestId = ++markRequestId;
    markLoading = true;
    try {
      const response = await getSuggestions(surface);
      if (requestId !== markRequestId) return;
      const unique = new Map<string, DBSuggestion>();
      for (const suggestion of response.suggestions) {
        const key = suggestion.query_text.toLocaleLowerCase("de");
        const existing = unique.get(key);
        if (!existing || (existing.entry_id <= 0 && suggestion.entry_id > 0)) unique.set(key, suggestion);
      }
      markSuggestions = [...unique.values()].slice(0, 6);
    } catch (cause) {
      if (requestId === markRequestId) markError = errorMessage(cause);
    } finally {
      if (requestId === markRequestId) markLoading = false;
    }
  }

  async function chooseMarkCandidate(candidate: DBSuggestion) {
    const target = markTarget;
    const current = article;
    if (!target || !current) return;
    markSaving = true;
    markError = "";
    try {
      if (candidate.entry_id <= 0) await analyzeWord(candidate.query_text);
      const occurrenceIndex = current.segments.slice(0, target.segmentIndex).filter((segment) => segment.kind === "bookmark").length;
      const body = insertReadingBookmark(current.body, current.segments, target, candidate.query_text);
      const next = await saveReadingArticle(current.id, { title: current.title, body, folder_id: current.folder_id });
      article = next;
      closeMarkPicker();
      const bookmark = next.segments.find((segment) => segment.kind === "bookmark" && segment.occurrence_index === occurrenceIndex);
      if (bookmark?.kind === "bookmark") selectWord(bookmark);
      window.getSelection()?.removeAllRanges();
    } catch (cause) {
      markError = errorMessage(cause);
    } finally {
      markSaving = false;
    }
  }

  function closeMarkPicker() {
    markRequestId += 1;
    markTarget = null;
    markSuggestions = [];
    markLoading = false;
    markError = "";
  }

  function handleWindowPointerDown(event: PointerEvent) {
    const element = event.target instanceof Element ? event.target : null;
    if (markTarget && !element?.closest(".reading-mark-picker")) closeMarkPicker();
  }

  function readingTextTarget(event: MouseEvent): ReadingTextTarget | null {
    const selected = targetFromSelection();
    if (selected) return selected;
    const caret = caretAtPoint(event.clientX, event.clientY);
    if (!caret) return null;
    const span = segmentSpan(caret.node);
    if (!span || span !== (event.target instanceof Element ? event.target.closest("[data-reading-segment]") : null)) return null;
    const text = span.textContent ?? "";
    let start = caret.offset;
    let end = caret.offset;
    while (start > 0 && isWordCharacter(text[start - 1])) start -= 1;
    while (end < text.length && isWordCharacter(text[end])) end += 1;
    return buildTextTarget(span, start, end);
  }

  function targetFromSelection() {
    const selection = window.getSelection();
    if (!selection || selection.isCollapsed || selection.rangeCount === 0) return null;
    const range = selection.getRangeAt(0);
    const startSpan = segmentSpan(range.startContainer);
    const endSpan = segmentSpan(range.endContainer);
    if (!startSpan || startSpan !== endSpan) return null;
    const start = textOffset(startSpan, range.startContainer, range.startOffset);
    const end = textOffset(startSpan, range.endContainer, range.endOffset);
    return buildTextTarget(startSpan, start, end);
  }

  function buildTextTarget(span: HTMLElement, rawStart: number, rawEnd: number) {
    const text = span.textContent ?? "";
    let start = Math.min(rawStart, rawEnd);
    let end = Math.max(rawStart, rawEnd);
    while (start < end && !isWordCharacter(text[start])) start += 1;
    while (end > start && !isWordCharacter(text[end - 1])) end -= 1;
    const surface = text.slice(start, end);
    if (!surface || [...surface].some((character) => !isWordCharacter(character))) return null;
    const segmentIndex = Number(span.dataset.readingSegment);
    if (!Number.isSafeInteger(segmentIndex)) return null;
    return { segmentIndex, start, end, surface };
  }

  function segmentSpan(node: Node) {
    const element = node instanceof Element ? node : node.parentElement;
    return element?.closest<HTMLElement>("[data-reading-segment]") ?? null;
  }

  function textOffset(root: HTMLElement, node: Node, offset: number) {
    const range = document.createRange();
    range.selectNodeContents(root);
    range.setEnd(node, offset);
    return range.toString().length;
  }

  function caretAtPoint(x: number, y: number) {
    if (document.caretPositionFromPoint) {
      const position = document.caretPositionFromPoint(x, y);
      if (position) return { node: position.offsetNode, offset: position.offset };
    }
    const legacy = (document as Document & { caretRangeFromPoint?: (x: number, y: number) => Range | null }).caretRangeFromPoint?.(x, y);
    return legacy ? { node: legacy.startContainer, offset: legacy.startOffset } : null;
  }

  function isWordCharacter(character: string | undefined) {
    return Boolean(character && /[\p{L}\p{M}'’-]/u.test(character));
  }

  function errorMessage(cause: unknown) {
    return cause instanceof Error ? cause.message : "操作失败";
  }
</script>

<svelte:window onpointerdown={handleWindowPointerDown} />

<div class="reading-view">
  {#if loading}
    <div class="article-state"><i class="ph ph-spinner-gap spin"></i></div>
  {:else if error || !article}
    <div class="article-state"><i class="ph ph-warning-circle"></i><span>{error || "文章不存在"}</span><a href="/learn/articles">返回文章目录</a></div>
  {:else}
    <header class="page-head">
      <div class="title-group">
        <a class="back-button" href="/learn/articles" aria-label="返回文章目录"><i class="ph ph-arrow-left"></i></a>
        <div><span>{folderPath}</span><h1>{article.title}</h1></div>
      </div>
      <div class="head-actions">
        <div class="bookmark-menu">
          <button class:active={showBookmarks} onclick={() => showBookmarks = !showBookmarks}>
            <i class="ph ph-bookmarks"></i><span class="bookmark-label">标记词</span><span class="bookmark-count">{bookmarks.length}</span>
          </button>
          {#if showBookmarks}
            <div class="bookmark-popover">
              {#each bookmarks as bookmark}
                <button onclick={() => jumpToBookmark(bookmark)}>
                  <strong>{bookmark.form_of}</strong><span>原文：{bookmark.surface}</span>
                </button>
              {/each}
            </div>
          {/if}
        </div>
        <button class="mobile-lookup" onclick={() => railOpen = true}><i class="ph ph-magnifying-glass"></i> 查词</button>
      </div>
    </header>

    <div class="reading-shell">
      <main class="article-pane">
        <article class="article-copy" oncontextmenu={handleArticleContextMenu}>
          {#each article.segments as segment, segmentIndex}
            {#if segment.kind === "text"}
              <span data-reading-segment={segmentIndex}>{segment.text}</span>
            {:else}
              <button
                id={`mark-${segment.occurrence_index}`}
                class:current={selectedOccurrence === segment.occurrence_index}
                class:unavailable={!segment.entry_id}
                class="marked"
                title={segment.entry_id ? segment.form_of : `${segment.form_of} 的词条已不可用`}
                onclick={() => selectWord(segment)}
              >{segment.surface}</button>
            {/if}
          {/each}
        </article>

        <footer class="article-foot">
          <span>{wordCount()} Wörter</span><span>{bookmarks.length} 标记</span>
        </footer>
      </main>

      <ReadingLookupRail {selectedKey} {selectedEntryId} open={railOpen} onClose={() => railOpen = false} />
    </div>

    {#if markTarget}
      <ReadingBookmarkPicker
        surface={markTarget.surface}
        x={markTarget.x}
        y={markTarget.y}
        suggestions={markSuggestions}
        loading={markLoading}
        saving={markSaving}
        error={markError}
        onChoose={(suggestion) => void chooseMarkCandidate(suggestion)}
        onClose={closeMarkPicker}
      />
    {/if}
  {/if}
</div>

<style>
  :global(.page-container:has(.reading-view)) { max-width: 1320px; }
  .reading-view { position: relative; display: grid; height: calc(100dvh - 4rem - 70px); grid-template-rows: auto minmax(0, 1fr); gap: 1rem; color: var(--text-main); }
  .article-state { display: flex; min-height: calc(100dvh - 8rem); align-items: center; justify-content: center; gap: 0.65rem; color: var(--text-muted); }
  .article-state a { color: var(--accent-main); text-decoration: none; }
  .page-head { display: flex; min-height: 62px; align-items: center; justify-content: space-between; gap: 1.5rem; padding-right: 88px; }
  .title-group { display: flex; align-items: center; gap: 0.9rem; min-width: 0; }
  .back-button { display: grid; width: 38px; height: 38px; flex: 0 0 auto; place-items: center; border: 1px solid var(--border-color); border-radius: 50%; background: var(--card-bg); color: var(--text-main); text-decoration: none; box-shadow: var(--shadow-sm); }
  .title-group span { color: var(--text-muted); font-size: 0.72rem; font-weight: 750; letter-spacing: 0.04em; }
  h1 { margin: 0.15rem 0 0; overflow: hidden; font-size: 1.45rem; line-height: 1.2; text-overflow: ellipsis; white-space: nowrap; }
  .head-actions { display: flex; align-items: center; gap: 0.55rem; }
  .bookmark-menu { position: relative; }
  .bookmark-menu > button, .mobile-lookup { display: flex; align-items: center; gap: 0.5rem; min-height: 40px; padding: 0 0.85rem; border: 1px solid var(--border-color); border-radius: var(--radius-full); background: var(--card-bg); color: var(--text-main); font-weight: 750; box-shadow: var(--shadow-sm); }
  .bookmark-menu > button.active { border-color: color-mix(in srgb, var(--accent-main) 55%, var(--border-color)); }
  .bookmark-menu > button .bookmark-count { display: grid; min-width: 21px; height: 21px; place-items: center; border-radius: 50%; background: var(--btn-secondary); color: var(--text-muted); font-size: 0.7rem; }
  .bookmark-popover { position: absolute; top: calc(100% + 0.6rem); right: 0; z-index: 230; width: 245px; padding: 0.45rem; border: 1px solid var(--border-color); border-radius: var(--radius-md); background: var(--card-bg); box-shadow: var(--shadow-hover); }
  .bookmark-popover button { display: flex; width: 100%; align-items: center; justify-content: space-between; gap: 0.8rem; padding: 0.7rem 0.75rem; border-radius: 9px; background: transparent; color: var(--text-main); text-align: left; }
  .bookmark-popover button:hover { background: var(--btn-secondary); }
  .bookmark-popover strong { font-size: 0.86rem; }
  .bookmark-popover span { overflow: hidden; color: var(--text-muted); font-size: 0.72rem; text-overflow: ellipsis; white-space: nowrap; }
  .mobile-lookup { display: none; }

  .head-actions { position: absolute; top: calc(62px + 1.85rem); right: calc(380px + 1rem); z-index: 4; }
  .bookmark-menu > button { min-height: 36px; padding: 0 0.7rem; background: color-mix(in srgb, var(--card-bg) 92%, transparent); }

  .reading-shell { display: grid; min-height: 0; grid-template-columns: minmax(0, 1fr) 380px; grid-template-rows: minmax(0, 1fr); overflow: hidden; border: 1px solid var(--border-color); border-radius: var(--radius-lg); background: var(--card-bg); box-shadow: var(--shadow-sm); }
  .article-pane { display: flex; min-width: 0; min-height: 0; flex-direction: column; overflow-y: auto; overscroll-behavior: contain; }
  .article-copy { width: min(720px, calc(100% - 5rem)); margin: auto; padding: clamp(4rem, 10vh, 7rem) 0; white-space: pre-wrap; font-family: "Avenir Next", Avenir, "Segoe UI", sans-serif; font-size: clamp(1.3rem, 1.65vw, 1.6rem); font-weight: 540; line-height: 2; letter-spacing: 0.005em; }
  .marked { position: relative; display: inline; padding: 0 0.04em 0.04em; border-radius: 0; background: transparent; color: inherit; font: inherit; font-weight: 690; transition: background 120ms ease, text-decoration-color 120ms ease; }
  .marked { box-shadow: inset 0 -2px color-mix(in srgb, var(--accent-main) 58%, transparent); }
  .marked::after { content: ""; position: absolute; left: calc(100% + 3px); bottom: -1px; width: 5px; height: 5px; border-radius: 50%; background: color-mix(in srgb, var(--accent-main) 72%, transparent); transition: background 120ms ease, transform 120ms ease; }
  .marked:hover, .marked.current { box-shadow: inset 0 -3px var(--accent-main); }
  .marked:hover::after, .marked.current::after { background: var(--accent-main); transform: scale(1.16); }
  .marked.unavailable { color: var(--text-muted); box-shadow: inset 0 -2px color-mix(in srgb, var(--text-muted) 42%, transparent); }
  .marked.unavailable::after { background: color-mix(in srgb, var(--text-muted) 62%, transparent); }
  .article-foot { display: flex; justify-content: space-between; margin: 0 2rem; padding: 1rem 0; border-top: 1px solid var(--border-color); color: var(--text-muted); font-size: 0.72rem; font-weight: 750; }

  @media (max-width: 900px) {
    :global(.page-container:has(.reading-view)) { max-width: 100%; }
    .reading-view { height: auto; min-height: calc(100dvh - 4rem); }
    .page-head { padding-right: 82px; }
    .head-actions { position: static; }
    .reading-shell { display: block; min-height: calc(100dvh - 8.8rem); }
    .article-pane { min-height: calc(100dvh - 9rem); }
    .article-copy { width: min(680px, calc(100% - 2.5rem)); padding: 3.5rem 0; }
    .mobile-lookup { display: flex; }
  }

  @media (max-width: 600px) {
    .reading-view { min-height: calc(100dvh - 4rem); gap: 0.6rem; }
    .page-head { min-height: 66px; align-items: center; padding: 0.55rem 0.65rem; }
    .back-button { display: none; }
    .title-group span { font-size: 0.64rem; }
    h1 { font-size: 1.15rem; }
    .bookmark-menu > button { padding: 0 0.65rem; font-size: 0.78rem; }
    .bookmark-menu > button i { display: none; }
    .mobile-lookup { width: 40px; padding: 0; justify-content: center; gap: 0; color: transparent; font-size: 0; }
    .mobile-lookup i { color: var(--text-main); font-size: 1rem; }
    .reading-shell { border-radius: var(--radius-md); }
    .article-copy { width: calc(100% - 2rem); font-size: 1.2rem; line-height: 1.95; }
    .article-foot { margin: 0 1rem; }
  }
</style>
