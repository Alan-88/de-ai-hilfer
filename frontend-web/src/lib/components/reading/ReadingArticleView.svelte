<script lang="ts">
  import ReadingBookmarkPicker from "$lib/components/reading/ReadingBookmarkPicker.svelte";
  import ReadingImportDialog from "$lib/components/reading/ReadingImportDialog.svelte";
  import ReadingLookupRail from "$lib/components/reading/ReadingLookupRail.svelte";
  import {
    createReadingTextAnchor,
    insertReadingBookmark,
    removeReadingBookmark,
    resolveReadingTextAnchor,
    type ReadingTextAnchor,
    type ReadingTextTarget,
  } from "$lib/readingBookmarks";
  import {
    getReadingArticle,
    getReadingIndex,
    markReadingArticleOpened,
    saveReadingArticle,
    type ReadingArticleDetail,
    type ReadingContentSegment,
    type ReadingFolder,
  } from "$lib/readingApi";
  import { analyzeWord, getSuggestions } from "$lib/queryApi";
  import type { DBSuggestion } from "$lib/types";

  type MarkTarget = ReadingTextAnchor & { requestId: number; x: number; y: number };

  let { articleId } = $props<{ articleId: number }>();

  let article = $state<ReadingArticleDetail | null>(null);
  let folders = $state<ReadingFolder[]>([]);
  let selectedKey = $state("");
  let selectedEntryId = $state<number | null>(null);
  let selectionVersion = $state(0);
  let selectedOccurrence = $state<number | null>(null);
  let showBookmarks = $state(false);
  let railOpen = $state(false);
  let loading = $state(true);
  let error = $state("");
  let markTarget = $state<MarkTarget | null>(null);
  let markSuggestions = $state<DBSuggestion[]>([]);
  let markLoading = $state(false);
  let markSavingRequestId = $state<number | null>(null);
  let markError = $state("");
  let bookmarkError = $state("");
  let bookmarkRemoving = $state<number | null>(null);
  let editOpen = $state(false);
  let markRequestId = 0;
  const pendingMarkAnalyses = new Map<string, Promise<void>>();
  let bookmarkSaveQueue: Promise<void> = Promise.resolve();

  const bookmarks = $derived(
    article?.segments.filter((segment): segment is Extract<ReadingContentSegment, { kind: "bookmark" }> => segment.kind === "bookmark") ?? [],
  );
  const folderPath = $derived(article ? buildFolderPath(article.folder_id, folders) : "");

  $effect(() => { void loadArticle(articleId); });

  async function loadArticle(id: number) {
    loading = true;
    error = "";
    try {
      const [next, index] = await Promise.all([getReadingArticle(id), getReadingIndex()]);
      article = next;
      folders = index.folders;
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
    // Re-selecting the same bookmark must restore it after a manual lookup.
    selectionVersion += 1;
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

  async function removeBookmark(bookmark: Extract<ReadingContentSegment, { kind: "bookmark" }>) {
    const current = article;
    if (!current || bookmarkRemoving !== null) return;
    bookmarkRemoving = bookmark.occurrence_index;
    bookmarkError = "";
    try {
      const body = removeReadingBookmark(current.body, current.segments, bookmark.occurrence_index);
      const next = await saveReadingArticle(current.id, { title: current.title, body, folder_id: current.folder_id });
      article = next;
      const remaining = next.segments.filter(
        (segment): segment is Extract<ReadingContentSegment, { kind: "bookmark" }> => segment.kind === "bookmark",
      );
      if (selectedOccurrence === bookmark.occurrence_index) {
        const replacement = remaining[Math.min(bookmark.occurrence_index, remaining.length - 1)];
        if (replacement) selectWord(replacement, false);
        else {
          selectedKey = "";
          selectedEntryId = null;
          selectedOccurrence = null;
        }
      } else if (selectedOccurrence !== null && selectedOccurrence > bookmark.occurrence_index) {
        selectedOccurrence -= 1;
      }
      if (remaining.length === 0) showBookmarks = false;
    } catch (cause) {
      bookmarkError = errorMessage(cause);
    } finally {
      bookmarkRemoving = null;
    }
  }

  function applyEditedArticle(next: ReadingArticleDetail) {
    article = next;
    editOpen = false;
    const selected = next.segments.find(
      (segment) => segment.kind === "bookmark"
        && segment.occurrence_index === selectedOccurrence
        && segment.entry_id === selectedEntryId,
    ) ?? next.segments.find(
      (segment) => segment.kind === "bookmark" && segment.entry_id === selectedEntryId,
    );
    if (selected?.kind === "bookmark") {
      selectWord(selected, false);
    } else {
      selectedKey = "";
      selectedEntryId = null;
      selectedOccurrence = null;
    }
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
    const requestId = ++markRequestId;
    markTarget = {
      ...createReadingTextAnchor(article.segments, target),
      requestId,
      x: Math.max(12, Math.min(event.clientX, window.innerWidth - 322)),
      y: Math.max(12, Math.min(event.clientY, window.innerHeight - 360)),
    };
    markSuggestions = [];
    markError = "";
    void loadMarkSuggestions(target.surface, requestId);
  }

  async function loadMarkSuggestions(surface: string, requestId: number) {
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
    const articleId = article?.id;
    if (!target || !articleId || markSavingRequestId === target.requestId) return;
    const requestId = target.requestId;
    markSavingRequestId = requestId;
    markError = "";
    try {
      if (candidate.entry_id <= 0) await ensureMarkAnalysis(candidate.query_text);

      const save = bookmarkSaveQueue.then(async () => {
        const current = article;
        if (!current || current.id !== articleId) throw new Error("文章已切换，请重新标记");
        const currentTarget = resolveReadingTextAnchor(current.segments, target);
        const occurrenceIndex = current.segments.slice(0, currentTarget.segmentIndex)
          .filter((segment) => segment.kind === "bookmark").length;
        const body = insertReadingBookmark(current.body, current.segments, currentTarget, candidate.query_text);
        const next = await saveReadingArticle(current.id, { title: current.title, body, folder_id: current.folder_id });
        if (selectedOccurrence !== null && selectedOccurrence >= occurrenceIndex) selectedOccurrence += 1;
        article = next;
        return next.segments.find(
          (segment) => segment.kind === "bookmark" && segment.occurrence_index === occurrenceIndex,
        );
      });
      bookmarkSaveQueue = save.then(() => undefined, () => undefined);
      const bookmark = await save;

      if (markTarget?.requestId === requestId) {
        closeMarkPicker();
        if (bookmark?.kind === "bookmark") selectWord(bookmark);
        window.getSelection()?.removeAllRanges();
      }
    } catch (cause) {
      const message = errorMessage(cause);
      if (markTarget?.requestId === requestId) markError = message;
      else bookmarkError = `${target.surface} 标记失败：${message}`;
    } finally {
      if (markSavingRequestId === requestId) markSavingRequestId = null;
    }
  }

  async function ensureMarkAnalysis(query: string) {
    const key = query.trim().toLocaleLowerCase("de");
    let pending = pendingMarkAnalyses.get(key);
    if (!pending) {
      pending = analyzeWord(query).then(() => undefined).finally(() => pendingMarkAnalyses.delete(key));
      pendingMarkAnalyses.set(key, pending);
    }
    await pending;
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
    if (showBookmarks && !element?.closest(".bookmark-menu")) showBookmarks = false;
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
    <div class="reading-shell">
      <main class="article-pane">
        <header class="page-head">
          <div class="title-group">
            <a class="back-button" href="/learn/articles" aria-label="返回文章目录"><i class="ph ph-arrow-left"></i></a>
            <div class="title-copy">
              <span class="folder-path" title={folderPath}>{folderPath}</span>
              <div class="title-line">
                <h1 title={article.title}>{article.title}</h1>
                <button class="edit-article" aria-label="编辑文章" title="编辑文章" onclick={() => editOpen = true}><i class="ph ph-pencil-simple"></i></button>
              </div>
            </div>
          </div>
        </header>

        <div class="article-scroll">
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
        </div>

        <footer class="article-foot">
          <span>{wordCount()} Wörter</span>
          <div class="foot-actions">
            <button class="mobile-lookup" aria-label="快捷查词" title="快捷查词" onclick={() => railOpen = true}><i class="ph ph-magnifying-glass"></i></button>
            <div class="bookmark-menu">
              <button class:active={showBookmarks} aria-label="标记词" aria-expanded={showBookmarks} aria-controls="article-bookmarks" disabled={bookmarks.length === 0} onclick={() => showBookmarks = !showBookmarks}>
                <i class="ph ph-bookmarks"></i><span>{bookmarks.length} 标记</span>
              </button>
              {#if showBookmarks}
                <div id="article-bookmarks" class="bookmark-popover">
                  {#each bookmarks as bookmark}
                    <div class="bookmark-row">
                      <button class="bookmark-jump" onclick={() => jumpToBookmark(bookmark)}>
                        <strong>{bookmark.form_of}</strong><span>原文：{bookmark.surface}</span>
                      </button>
                      <button
                        class="bookmark-remove"
                        aria-label={`取消标记 ${bookmark.surface}`}
                        title="取消标记"
                        disabled={bookmarkRemoving !== null}
                        onclick={() => void removeBookmark(bookmark)}
                      ><i class={bookmarkRemoving === bookmark.occurrence_index ? "ph ph-spinner-gap spin" : "ph ph-trash"}></i></button>
                    </div>
                  {/each}
                  {#if bookmarkError}<div class="bookmark-error" role="alert">{bookmarkError}</div>{/if}
                </div>
              {/if}
            </div>
          </div>
        </footer>
      </main>

      {#if railOpen}
        <button class="rail-dismiss" aria-label="关闭查词栏" onclick={() => railOpen = false}></button>
      {/if}
      <ReadingLookupRail {selectedKey} {selectedEntryId} {selectionVersion} open={railOpen} onClose={() => railOpen = false} />
    </div>

    {#if markTarget}
      <ReadingBookmarkPicker
        surface={markTarget.surface}
        x={markTarget.x}
        y={markTarget.y}
        suggestions={markSuggestions}
        loading={markLoading}
        saving={markSavingRequestId === markTarget.requestId}
        error={markError}
        onChoose={(suggestion) => void chooseMarkCandidate(suggestion)}
        onClose={closeMarkPicker}
      />
    {/if}
    {#if editOpen}
      <ReadingImportDialog
        folders={folders.map((folder) => ({ id: folder.id, name: buildFolderPath(folder.id, folders) }))}
        defaultFolderId={article.folder_id}
        articleId={article.id}
        onClose={() => editOpen = false}
        onCreated={applyEditedArticle}
      />
    {/if}
  {/if}
</div>

<style>
  :global(.page-container:has(.reading-view)) { max-width: 1320px; }
  .reading-view { position: relative; display: grid; height: calc(100dvh - 4rem - 70px); grid-template-rows: minmax(0, 1fr); color: var(--text-main); }
  .article-state { display: flex; min-height: calc(100dvh - 8rem); align-items: center; justify-content: center; gap: 0.65rem; color: var(--text-muted); }
  .article-state a { color: var(--accent-main); text-decoration: none; }
  .page-head { z-index: 5; display: flex; flex: 0 0 auto; min-height: 75px; align-items: center; padding: 0.65rem 1.35rem; border-bottom: 1px solid var(--border-color); background: var(--card-bg); }
  .title-group { display: flex; width: 100%; align-items: center; gap: 0.9rem; min-width: 0; }
  .title-copy { min-width: 0; flex: 1; }
  .folder-path { display: block; overflow: hidden; color: var(--text-muted); font-size: 0.72rem; font-weight: 750; letter-spacing: 0.04em; text-overflow: ellipsis; white-space: nowrap; }
  .title-line { display: flex; min-width: 0; align-items: center; gap: 0.35rem; }
  .back-button { display: grid; width: 38px; height: 38px; flex: 0 0 auto; place-items: center; border: 1px solid var(--border-color); border-radius: 50%; background: var(--control-bg); color: var(--text-main); text-decoration: none; box-shadow: var(--shadow-sm); }
  h1 { margin: 0; min-width: 0; overflow: hidden; font-size: 1.45rem; line-height: 1.2; text-overflow: ellipsis; white-space: nowrap; }
  .edit-article { display: grid; width: 30px; height: 30px; flex: 0 0 auto; place-items: center; margin-top: 0.12rem; border-radius: 50%; background: transparent; color: var(--text-muted); }
  .edit-article:hover, .edit-article:focus-visible { background: var(--btn-secondary); color: var(--text-main); outline: none; }
  .foot-actions { display: flex; align-items: center; gap: 6px; }
  .bookmark-menu > button, .mobile-lookup { display: flex; height: 32px; align-items: center; justify-content: center; gap: 5px; padding: 0 9px; border: 1px solid var(--border-color); border-radius: var(--radius-full); background: var(--control-bg); color: var(--text-muted); font-size: 11px; font-weight: 750; white-space: nowrap; }
  .bookmark-menu > button i, .mobile-lookup i { font-size: 14px; }
  .bookmark-menu > button:hover, .mobile-lookup:hover { background: var(--btn-secondary); color: var(--text-main); }
  .bookmark-menu > button:focus-visible, .mobile-lookup:focus-visible { outline: 2px solid var(--accent-main); outline-offset: 2px; }
  .bookmark-menu > button.active { border-color: color-mix(in srgb, var(--accent-main) 55%, var(--border-color)); }
  .bookmark-menu > button:disabled { cursor: default; opacity: 0.55; }
  .bookmark-popover { position: absolute; bottom: 52px; right: 12px; z-index: 230; width: min(280px, calc(100% - 24px)); max-height: min(380px, calc(100% - 130px)); overflow-y: auto; overscroll-behavior: contain; padding: 0.45rem; border: 1px solid var(--border-color); border-radius: var(--radius-md); background: var(--card-bg); box-shadow: var(--shadow-hover); }
  .bookmark-row { display: grid; grid-template-columns: minmax(0, 1fr) 34px; align-items: center; gap: 0.2rem; border-radius: 9px; }
  .bookmark-row:hover { background: var(--btn-secondary); }
  .bookmark-jump { display: flex; width: 100%; min-width: 0; align-items: center; justify-content: space-between; gap: 0.8rem; padding: 0.7rem 0.55rem 0.7rem 0.75rem; background: transparent; color: var(--text-main); text-align: left; }
  .bookmark-remove { display: grid; width: 30px; height: 30px; place-items: center; border-radius: 50%; background: transparent; color: var(--text-muted); }
  .bookmark-remove:hover, .bookmark-remove:focus-visible { background: color-mix(in srgb, #c64b43 10%, var(--card-bg)); color: var(--danger-text, #b8453f); outline: none; }
  .bookmark-remove:disabled { opacity: 0.55; }
  .bookmark-popover strong { font-size: 0.86rem; }
  .bookmark-popover span { overflow: hidden; color: var(--text-muted); font-size: 0.72rem; text-overflow: ellipsis; white-space: nowrap; }
  .bookmark-error { margin: 0.35rem 0.45rem 0.15rem; color: var(--danger-text, #b8453f); font-size: 0.7rem; line-height: 1.45; }
  .mobile-lookup { display: none; }

  .reading-shell { position: relative; display: grid; min-height: 0; grid-template-columns: minmax(0, 1fr) 380px; grid-template-rows: minmax(0, 1fr); overflow: hidden; border: 1px solid var(--border-color); border-radius: var(--radius-lg); background: var(--card-bg); box-shadow: var(--shadow-sm); }
  .article-pane { position: relative; display: flex; min-width: 0; min-height: 0; flex-direction: column; overflow: hidden; }
  .article-scroll { display: flex; min-height: 0; flex: 1; overflow-y: auto; overscroll-behavior: contain; }
  .article-copy { width: min(720px, calc(100% - 5rem)); flex: 0 0 auto; margin: auto; padding: clamp(4rem, 10vh, 7rem) 0; white-space: pre-wrap; font-family: "Avenir Next", Avenir, "Segoe UI", sans-serif; font-size: clamp(1.3rem, 1.65vw, 1.6rem); font-weight: 540; line-height: 2; letter-spacing: 0.005em; }
  .marked { position: relative; display: inline; padding: 0 0.04em 0.04em; border-radius: 0; background: transparent; color: inherit; font: inherit; font-weight: 690; transition: background 120ms ease, text-decoration-color 120ms ease; }
  .marked { box-shadow: inset 0 -2px color-mix(in srgb, var(--accent-main) 58%, transparent); }
  .marked::after { content: ""; position: absolute; left: calc(100% + 3px); bottom: -1px; width: 5px; height: 5px; border-radius: 50%; background: color-mix(in srgb, var(--accent-main) 72%, transparent); transition: background 120ms ease, transform 120ms ease; }
  .marked:hover, .marked.current { box-shadow: inset 0 -3px var(--accent-main); }
  .marked:hover::after, .marked.current::after { background: var(--accent-main); transform: scale(1.16); }
  .marked.unavailable { color: var(--text-muted); box-shadow: inset 0 -2px color-mix(in srgb, var(--text-muted) 42%, transparent); }
  .marked.unavailable::after { background: color-mix(in srgb, var(--text-muted) 62%, transparent); }
  .article-foot { display: flex; height: 44px; flex: 0 0 auto; align-items: center; justify-content: space-between; gap: 8px; padding: 0 12px; border-top: 1px solid var(--border-color); background: var(--card-bg); color: var(--text-muted); font-size: 11px; font-weight: 750; }
  .rail-dismiss { display: none; }

  @media (max-width: 900px) {
    :global(.page-container:has(.reading-view)) { max-width: 100%; }
    .reading-shell { display: block; height: 100%; }
    .article-pane { height: 100%; }
    .page-head { min-height: 66px; }
    .folder-path { display: none; }
    .article-copy { width: min(680px, calc(100% - 2.5rem)); padding: 3.5rem 0; }
    .mobile-lookup { display: flex; }
    .rail-dismiss { position: absolute; inset: 0; z-index: 20; display: block; width: 100%; height: 100%; border-radius: inherit; background: color-mix(in srgb, var(--text-main) 8%, transparent); cursor: default; }
  }

  @media (max-width: 768px) {
    .reading-view { height: calc(100dvh - 179px - env(safe-area-inset-bottom)); min-height: 0; }
  }

  @media (max-width: 600px) {
    .page-head { min-height: 56px; padding: 0.55rem 0.65rem; }
    .title-group { gap: 0.6rem; }
    .back-button { width: 34px; height: 34px; }
    h1 { display: -webkit-box; -webkit-box-orient: vertical; -webkit-line-clamp: 2; line-clamp: 2; font-size: 1.15rem; line-height: 1.25; white-space: normal; overflow-wrap: anywhere; }
    .mobile-lookup { width: 32px; padding: 0; }
    .reading-shell { border-radius: var(--radius-md); }
    .article-copy { width: calc(100% - 2rem); font-size: 1.125rem; line-height: 1.9; }
  }
</style>
