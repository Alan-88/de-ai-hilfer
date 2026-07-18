<script lang="ts">
  import { goto } from "$app/navigation";
  import DirectoryArticleList from "$lib/components/reading-directory/DirectoryArticleList.svelte";
  import DirectoryContextMenu from "$lib/components/reading-directory/DirectoryContextMenu.svelte";
  import DirectoryFolderTree from "$lib/components/reading-directory/DirectoryFolderTree.svelte";
  import DirectoryMoveDialog from "$lib/components/reading-directory/DirectoryMoveDialog.svelte";
  import ReadingImportDialog from "$lib/components/reading/ReadingImportDialog.svelte";
  import {
    createReadingFolder,
    deleteReadingArticle,
    deleteReadingFolder,
    getReadingIndex,
    moveReadingArticle,
    renameReadingArticle,
    setReadingArticleStatus,
    updateReadingFolder,
  } from "$lib/readingApi";
  import { onMount } from "svelte";

  type Folder = { id: number; name: string; parentId: number | null };
  type DragItem = { kind: "folder" | "article"; id: number };
  type SmartView = "all" | "recent" | "unfinished" | "completed" | "folder";
  type ContextTarget = { kind: "folder" | "article"; id: number; x: number; y: number };
  type Article = {
    id: number;
    title: string;
    folderId: number | null;
    memory: number | null;
    marks: number;
    lastOpened: string;
    recency: number;
    completed: boolean;
  };

  let folders = $state<Folder[]>([]);
  let articles = $state<Article[]>([]);

  let expandedIds = $state<number[]>([]);
  let selectedFolderId = $state<number | null>(null);
  let selectedArticleId = $state<number | null>(null);
  let view = $state<SmartView>("all");
  let search = $state("");
  let dragItem = $state<DragItem | null>(null);
  let contextTarget = $state<ContextTarget | null>(null);
  let foldersOpen = $state(false);
  let importOpen = $state(false);
  let editingArticleId = $state<number | null>(null);
  let movingItem = $state<DragItem | null>(null);
  let movePending = $state(false);
  let loading = $state(true);
  let notice = $state("");
  let noticeTimer: number | undefined;
  let longPressTimer: number | undefined;
  let pressOrigin: { x: number; y: number } | null = null;

  onMount(() => { void loadIndex(); });

  async function loadIndex() {
    loading = true;
    try {
      const response = await getReadingIndex();
      folders = response.folders.map((folder) => ({ id: folder.id, name: folder.name, parentId: folder.parent_id }));
      articles = response.articles.map((article) => ({
        id: article.id,
        title: article.title,
        folderId: article.folder_id,
        memory: article.memory_percent,
        marks: article.bookmark_count,
        lastOpened: formatDate(article.last_opened_at),
        recency: Date.parse(article.last_opened_at ?? article.updated_at),
        completed: article.completed,
      }));
      if (expandedIds.length === 0) expandedIds = folders.filter((folder) => folder.parentId === null).map((folder) => folder.id);
    } catch (cause) {
      showNotice(errorMessage(cause));
    } finally {
      loading = false;
    }
  }

  const visibleArticles = $derived.by(() => {
    let result = [...articles];
    if (view === "folder" && selectedFolderId !== null) {
      const ids = descendantFolderIds(selectedFolderId);
      result = result.filter((article) => article.folderId !== null && ids.has(article.folderId));
    } else if (view === "unfinished") {
      result = result.filter((article) => !article.completed);
    } else if (view === "completed") {
      result = result.filter((article) => article.completed);
    }

    if (view === "recent") result.sort((left, right) => right.recency - left.recency);
    const term = search.trim().toLocaleLowerCase("de");
    if (term) {
      result = result.filter((article) => `${article.title} ${folderPath(article.folderId)}`.toLocaleLowerCase("de").includes(term));
    }
    return result.map((article) => ({ ...article, path: folderPath(article.folderId) || "未归类" }));
  });

  const viewTitle = $derived.by(() => {
    if (view === "folder") return folderPath(selectedFolderId) || "未归类";
    return { all: "全部文章", recent: "最近阅读", unfinished: "未读完", completed: "已完成" }[view];
  });

  function descendantFolderIds(folderId: number) {
    const ids = new Set([folderId]);
    let changed = true;
    while (changed) {
      changed = false;
      for (const folder of folders) {
        if (folder.parentId !== null && ids.has(folder.parentId) && !ids.has(folder.id)) {
          ids.add(folder.id);
          changed = true;
        }
      }
    }
    return ids;
  }

  function folderPath(folderId: number | null) {
    if (folderId === null) return "";
    const names: string[] = [];
    const seen = new Set<number>();
    let current = folders.find((folder) => folder.id === folderId);
    while (current && !seen.has(current.id)) {
      names.unshift(current.name);
      seen.add(current.id);
      current = current.parentId === null ? undefined : folders.find((folder) => folder.id === current?.parentId);
    }
    return names.join(" / ");
  }

  function countForFolder(folderId: number) {
    const ids = descendantFolderIds(folderId);
    return articles.filter((article) => article.folderId !== null && ids.has(article.folderId)).length;
  }

  function selectSmart(next: Exclude<SmartView, "folder">) {
    view = next;
    selectedFolderId = null;
    foldersOpen = false;
  }

  function selectFolder(folderId: number) {
    view = "folder";
    selectedFolderId = folderId;
    foldersOpen = false;
  }

  function toggleFolder(folderId: number) {
    expandedIds = expandedIds.includes(folderId)
      ? expandedIds.filter((id) => id !== folderId)
      : [...expandedIds, folderId];
  }

  function showNotice(message: string) {
    notice = message;
    if (noticeTimer) window.clearTimeout(noticeTimer);
    noticeTimer = window.setTimeout(() => notice = "", 2200);
  }

  async function createFolder(parentId = view === "folder" ? selectedFolderId : null) {
    contextTarget = null;
    const name = window.prompt("文件夹名称")?.trim();
    if (!name) return;
    try {
      const folder = await createReadingFolder(name, parentId);
      folders = [...folders, { id: folder.id, name: folder.name, parentId: folder.parent_id }];
      if (parentId !== null && !expandedIds.includes(parentId)) expandedIds = [...expandedIds, parentId];
      selectFolder(folder.id);
    } catch (cause) {
      showNotice(errorMessage(cause));
    }
  }

  function startDrag(event: DragEvent, item: DragItem) {
    dragItem = item;
    event.stopPropagation();
    if (event.dataTransfer) {
      event.dataTransfer.effectAllowed = "move";
      event.dataTransfer.setData("text/plain", `${item.kind}:${item.id}`);
    }
  }

  function endDrag() {
    dragItem = null;
  }

  function isInsideFolder(targetId: number | null, movingId: number) {
    let currentId = targetId;
    const seen = new Set<number>();
    while (currentId !== null && !seen.has(currentId)) {
      if (currentId === movingId) return true;
      seen.add(currentId);
      currentId = folders.find((folder) => folder.id === currentId)?.parentId ?? null;
    }
    return false;
  }

  async function dropInto(folderId: number | null) {
    const item = dragItem;
    dragItem = null;
    if (!item) return;
    if (item.kind === "article") {
      const article = articles.find((candidate) => candidate.id === item.id);
      if (article && article.folderId !== folderId) {
        try {
          await moveReadingArticle(article.id, folderId);
          articles = articles.map((candidate) => candidate.id === article.id ? { ...candidate, folderId } : candidate);
          showNotice(folderId === null ? "文章已移到根目录" : `文章已移到 ${folderPath(folderId)}`);
        } catch (cause) {
          showNotice(errorMessage(cause));
        }
      }
    } else {
      const folder = folders.find((candidate) => candidate.id === item.id);
      if (folder && isInsideFolder(folderId, folder.id)) {
        showNotice("文件夹不能移动到自身或子文件夹中");
      } else if (folder && folder.parentId !== folderId) {
        try {
          await updateReadingFolder(folder.id, folder.name, folderId);
          folders = folders.map((candidate) => candidate.id === folder.id ? { ...candidate, parentId: folderId } : candidate);
          if (folderId !== null && !expandedIds.includes(folderId)) expandedIds = [...expandedIds, folderId];
          showNotice(folderId === null ? "文件夹已移到根目录" : `文件夹已移到 ${folderPath(folderId)}`);
        } catch (cause) {
          showNotice(errorMessage(cause));
        }
      }
    }
  }

  function allowStatusDrop(event: DragEvent) {
    if (dragItem?.kind !== "article") return;
    event.preventDefault();
    if (event.dataTransfer) event.dataTransfer.dropEffect = "move";
  }

  async function dropOnStatus(event: DragEvent, completed: boolean) {
    event.preventDefault();
    const item = dragItem;
    dragItem = null;
    if (item?.kind !== "article") return;
    const article = articles.find((candidate) => candidate.id === item.id);
    if (!article || article.completed === completed) return;
    try {
      await setReadingArticleStatus(article.id, completed);
      articles = articles.map((candidate) => candidate.id === article.id ? { ...candidate, completed } : candidate);
      showNotice(completed ? "文章已标为已完成" : "文章已移回未读完");
    } catch (cause) {
      showNotice(errorMessage(cause));
    }
  }

  function openContext(event: MouseEvent, kind: "folder" | "article", id: number) {
    event.preventDefault();
    event.stopPropagation();
    openContextAt(kind, id, event.clientX, event.clientY);
  }

  function openContextAt(kind: "folder" | "article", id: number, x: number, y: number) {
    const height = kind === "folder" ? 194 : 230;
    contextTarget = {
      kind,
      id,
      x: Math.max(8, Math.min(x, window.innerWidth - 184)),
      y: Math.max(8, Math.min(y, window.innerHeight - height - 8)),
    };
  }

  function startLongPress(event: PointerEvent, kind: "folder" | "article", id: number) {
    if (event.pointerType === "mouse") return;
    endLongPress();
    pressOrigin = { x: event.clientX, y: event.clientY };
    longPressTimer = window.setTimeout(() => {
      openContextAt(kind, id, event.clientX, event.clientY);
      longPressTimer = undefined;
    }, 560);
  }

  function moveLongPress(event: PointerEvent) {
    if (!pressOrigin || !longPressTimer) return;
    if (Math.hypot(event.clientX - pressOrigin.x, event.clientY - pressOrigin.y) > 8) endLongPress();
  }

  function endLongPress() {
    if (longPressTimer) window.clearTimeout(longPressTimer);
    longPressTimer = undefined;
    pressOrigin = null;
  }

  async function renameTarget() {
    const target = contextTarget;
    contextTarget = null;
    if (!target) return;
    if (target.kind === "folder") {
      const folder = folders.find((item) => item.id === target.id);
      if (!folder) return;
      const name = window.prompt("重命名文件夹", folder.name)?.trim();
      if (!name || name === folder.name) return;
      try {
        await updateReadingFolder(folder.id, name, folder.parentId);
        folders = folders.map((item) => item.id === folder.id ? { ...item, name } : item);
      } catch (cause) {
        showNotice(errorMessage(cause));
      }
    } else {
      const article = articles.find((item) => item.id === target.id);
      if (!article) return;
      const title = window.prompt("重命名文章", article.title)?.trim();
      if (!title || title === article.title) return;
      try {
        await renameReadingArticle(article.id, title);
        articles = articles.map((item) => item.id === article.id ? { ...item, title } : item);
      } catch (cause) {
        showNotice(errorMessage(cause));
      }
    }
  }

  async function deleteTarget() {
    const target = contextTarget;
    contextTarget = null;
    if (!target) return;
    if (target.kind === "folder") {
      const folder = folders.find((item) => item.id === target.id);
      if (!folder) return;
      const hasContent = folders.some((item) => item.parentId === folder.id) || articles.some((item) => item.folderId === folder.id);
      if (hasContent) {
        showNotice("非空文件夹不能直接删除");
        return;
      }
      if (window.confirm(`删除空文件夹“${folder.name}”？`)) {
        try {
          await deleteReadingFolder(folder.id);
          folders = folders.filter((item) => item.id !== folder.id);
          if (selectedFolderId === folder.id) selectSmart("all");
        } catch (cause) {
          showNotice(errorMessage(cause));
        }
      }
    } else {
      const article = articles.find((item) => item.id === target.id);
      if (article && window.confirm(`删除文章“${article.title}”？`)) {
        try {
          await deleteReadingArticle(article.id);
          articles = articles.filter((item) => item.id !== article.id);
          if (selectedArticleId === article.id) selectedArticleId = null;
        } catch (cause) {
          showNotice(errorMessage(cause));
        }
      }
    }
  }

  function editTarget() {
    if (contextTarget?.kind !== "article") return;
    editingArticleId = contextTarget.id;
    contextTarget = null;
  }

  async function toggleTargetStatus() {
    const target = contextTarget;
    contextTarget = null;
    if (target?.kind !== "article") return;
    const article = articles.find((item) => item.id === target.id);
    if (!article) return;
    try {
      await setReadingArticleStatus(article.id, !article.completed);
      articles = articles.map((item) => item.id === article.id ? { ...item, completed: !article.completed } : item);
    } catch (cause) {
      showNotice(errorMessage(cause));
    }
  }

  function moveTarget() {
    if (!contextTarget) return;
    movingItem = { kind: contextTarget.kind, id: contextTarget.id };
    contextTarget = null;
  }

  async function moveItemFromDialog(folderId: number | null) {
    const item = movingItem;
    if (!item) return;
    movePending = true;
    try {
      if (item.kind === "article") {
        const article = articles.find((candidate) => candidate.id === item.id);
        if (!article || article.folderId === folderId) return;
        await moveReadingArticle(article.id, folderId);
        articles = articles.map((candidate) => candidate.id === article.id ? { ...candidate, folderId } : candidate);
        showNotice(folderId === null ? "文章已移到未归类" : `文章已移到 ${folderPath(folderId)}`);
      } else {
        const folder = folders.find((candidate) => candidate.id === item.id);
        if (!folder || folder.parentId === folderId || isInsideFolder(folderId, folder.id)) return;
        await updateReadingFolder(folder.id, folder.name, folderId);
        folders = folders.map((candidate) => candidate.id === folder.id ? { ...candidate, parentId: folderId } : candidate);
        if (folderId !== null && !expandedIds.includes(folderId)) expandedIds = [...expandedIds, folderId];
        showNotice(folderId === null ? "文件夹已移到根目录" : `文件夹已移到 ${folderPath(folderId)}`);
      }
      movingItem = null;
    } catch (cause) {
      showNotice(errorMessage(cause));
    } finally {
      movePending = false;
    }
  }

  function handleWindowPointerDown(event: PointerEvent) {
    const element = event.target instanceof Element ? event.target : null;
    if (contextTarget && !element?.closest(".directory-context-menu")) contextTarget = null;
  }

  function handleWindowKeydown(event: KeyboardEvent) {
    if (event.key === "Escape") {
      contextTarget = null;
      foldersOpen = false;
      if (!movePending) movingItem = null;
    }
  }

  function formatDate(value: string | null) {
    if (!value) return "未打开";
    const date = new Date(value);
    const today = new Date();
    const day = new Date(date.getFullYear(), date.getMonth(), date.getDate()).getTime();
    const current = new Date(today.getFullYear(), today.getMonth(), today.getDate()).getTime();
    if (day === current) return "今天";
    if (day === current - 86_400_000) return "昨天";
    return `${date.getMonth() + 1}月${date.getDate()}日`;
  }

  function errorMessage(cause: unknown) {
    return cause instanceof Error ? cause.message : "操作失败";
  }
</script>

<svelte:window onpointerdown={handleWindowPointerDown} onkeydown={handleWindowKeydown} />

<div class="split-scheme">
  <main class:folders-open={foldersOpen}>
    <button class="mobile-scrim" aria-label="关闭文件夹" onclick={() => foldersOpen = false}></button>
    <aside>
      <div class="aside-title"><strong>文章</strong></div>
      <nav class="smart-views">
        <button class:active={view === "all"} onclick={() => selectSmart("all")}><i class="ph-fill ph-tray"></i><span>全部文章</span><em>{articles.length}</em></button>
        <button class:active={view === "recent"} onclick={() => selectSmart("recent")}><i class="ph ph-clock-counter-clockwise"></i><span>最近阅读</span></button>
        <button class:active={view === "unfinished"} ondragover={allowStatusDrop} ondrop={(event) => dropOnStatus(event, false)} onclick={() => selectSmart("unfinished")}><i class="ph ph-circle-half"></i><span>未读完</span><em>{articles.filter((article) => !article.completed).length}</em></button>
        <button class:active={view === "completed"} ondragover={allowStatusDrop} ondrop={(event) => dropOnStatus(event, true)} onclick={() => selectSmart("completed")}><i class="ph ph-check-circle"></i><span>已完成</span><em>{articles.filter((article) => article.completed).length}</em></button>
      </nav>

      <DirectoryFolderTree
        {folders}
        {expandedIds}
        {selectedFolderId}
        folderViewActive={view === "folder"}
        {dragItem}
        {countForFolder}
        onSelect={selectFolder}
        onToggle={toggleFolder}
        onCreate={() => createFolder()}
        onDragStart={startDrag}
        onDragEnd={endDrag}
        onDrop={dropInto}
        onContextMenu={openContext}
        onPressStart={startLongPress}
        onPressMove={moveLongPress}
        onPressEnd={endLongPress}
      />
    </aside>

    <section class="content">
      <div class="content-head">
        <div><h2>{viewTitle}</h2><span>{visibleArticles.length} 篇</span></div>
        <button onclick={() => importOpen = true}><i class="ph ph-plus"></i> 导入文章</button>
      </div>
      <div class="toolbar">
        <button class="mobile-folder-button" aria-label="打开文件夹" onclick={() => foldersOpen = true}><i class="ph ph-folders"></i></button>
        <form onsubmit={(event) => event.preventDefault()}><i class="ph ph-magnifying-glass"></i><input bind:value={search} placeholder="搜索文章或路径" /></form>
      </div>
      {#if loading}
        <div class="loading-list"><span></span><span></span><span></span></div>
      {:else}
        <DirectoryArticleList
          articles={visibleArticles}
          selectedId={selectedArticleId}
          onSelect={(id) => { selectedArticleId = id; void goto(`/learn/articles/${id}`); }}
          onDragStart={startDrag}
          onDragEnd={endDrag}
          onContextMenu={openContext}
          onPressStart={startLongPress}
          onPressMove={moveLongPress}
          onPressEnd={endLongPress}
        />
      {/if}
    </section>
  </main>

  {#if notice}<div class="notice" role="status">{notice}</div>{/if}
  <DirectoryContextMenu
    target={contextTarget}
    onRename={renameTarget}
    onEdit={editTarget}
    onToggleStatus={toggleTargetStatus}
    statusLabel={contextTarget?.kind === "article" && articles.find((article) => article.id === contextTarget?.id)?.completed ? "移回未读完" : "标为已完成"}
    onMove={moveTarget}
    onDelete={deleteTarget}
    onCreateChild={() => contextTarget?.kind === "folder" && createFolder(contextTarget.id)}
  />
  {#if importOpen}
    <ReadingImportDialog
      folders={folders.map((folder) => ({ id: folder.id, name: folderPath(folder.id) }))}
      defaultFolderId={view === "folder" ? selectedFolderId : null}
      onClose={() => importOpen = false}
      onCreated={(article) => { importOpen = false; void goto(`/learn/articles/${article.id}`); }}
    />
  {/if}
  {#if editingArticleId !== null}
    <ReadingImportDialog
      folders={folders.map((folder) => ({ id: folder.id, name: folderPath(folder.id) }))}
      defaultFolderId={null}
      articleId={editingArticleId}
      onClose={() => editingArticleId = null}
      onCreated={(article) => { editingArticleId = null; void loadIndex(); void goto(`/learn/articles/${article.id}`); }}
    />
  {/if}
  {#if movingItem}
    <DirectoryMoveDialog
      itemKind={movingItem.kind === "article" ? "文章" : "文件夹"}
      itemTitle={movingItem.kind === "article" ? articles.find((article) => article.id === movingItem?.id)?.title ?? "" : folders.find((folder) => folder.id === movingItem?.id)?.name ?? ""}
      folders={folders.filter((folder) => movingItem?.kind !== "folder" || !descendantFolderIds(movingItem.id).has(folder.id)).map((folder) => ({ id: folder.id, name: folderPath(folder.id) }))}
      currentFolderId={movingItem.kind === "article" ? articles.find((article) => article.id === movingItem?.id)?.folderId ?? null : folders.find((folder) => folder.id === movingItem?.id)?.parentId ?? null}
      moving={movePending}
      onSelect={(folderId) => void moveItemFromDialog(folderId)}
      onClose={() => { if (!movePending) movingItem = null; }}
    />
  {/if}
</div>

<style>
  :global(.page-container:has(.split-scheme)) { max-width: 1160px; }
  .split-scheme { color: var(--text-main); }
  main { position: relative; display: grid; min-height: 660px; grid-template-columns: 230px minmax(0, 1fr); overflow: hidden; border: 1px solid var(--border-color); border-radius: var(--radius-lg); background: var(--card-bg); box-shadow: var(--shadow-sm); }
  aside { z-index: 6; padding: 1.25rem 0.9rem; border-right: 1px solid var(--border-color); background: color-mix(in srgb, var(--card-bg) 76%, var(--bg-color)); }
  .aside-title { display: flex; min-height: 38px; align-items: center; padding: 0 0.55rem; }
  .aside-title strong { font-size: 0.9rem; }
  .smart-views { display: grid; gap: 0.18rem; }
  .smart-views button { display: grid; min-height: 38px; grid-template-columns: 20px 1fr auto; align-items: center; gap: 0.45rem; padding: 0 0.55rem; border-radius: 8px; background: transparent; color: var(--text-muted); text-align: left; }
  .smart-views button.active { background: var(--card-bg); color: var(--text-main); box-shadow: var(--shadow-sm); }
  .smart-views button span { overflow: hidden; font-size: 0.76rem; font-weight: 750; text-overflow: ellipsis; white-space: nowrap; }
  .smart-views em { font-size: 0.66rem; font-style: normal; }
  .content { min-width: 0; padding: 1.3rem; }
  .content-head, .content-head > div, .toolbar { display: flex; align-items: center; }
  .content-head { justify-content: space-between; }
  .content-head > div { min-width: 0; gap: 0.6rem; }
  .content-head h2 { overflow: hidden; margin: 0; font-size: 1.1rem; text-overflow: ellipsis; white-space: nowrap; }
  .content-head span { flex: 0 0 auto; color: var(--text-muted); font-size: 0.72rem; }
  .content-head > button { display: flex; min-height: 38px; flex: 0 0 auto; align-items: center; gap: 0.4rem; padding: 0 0.85rem; border-radius: var(--radius-full); background: var(--accent-main); color: white; font-weight: 800; }
  .toolbar { gap: 0.5rem; margin: 0.9rem 0; }
  form { display: flex; height: 40px; flex: 1; align-items: center; gap: 0.55rem; padding: 0 0.8rem; border: 1px solid var(--border-color); border-radius: var(--radius-full); background: var(--bg-color); }
  form i, .toolbar > button { color: var(--text-muted); }
  input { width: 100%; min-width: 0; border: 0; outline: 0; background: transparent; color: var(--text-main); font: inherit; }
  .toolbar > button { display: grid; width: 40px; height: 40px; flex: 0 0 auto; place-items: center; border-radius: 50%; background: var(--bg-color); }
  .mobile-folder-button { display: none !important; }
  .mobile-scrim { display: none; }
  .notice { position: fixed; left: 50%; bottom: 2rem; z-index: 520; transform: translateX(-50%); padding: 0.65rem 0.9rem; border: 1px solid var(--border-color); border-radius: var(--radius-full); background: var(--card-bg); color: var(--text-main); box-shadow: var(--shadow-hover); font-size: 0.76rem; font-weight: 750; }
  .loading-list { display: flex; min-height: 180px; align-items: center; justify-content: center; gap: 0.38rem; }
  .loading-list span { width: 7px; height: 7px; border-radius: 50%; background: var(--text-muted); animation: directory-pulse 850ms ease-in-out infinite alternate; }
  .loading-list span:nth-child(2) { animation-delay: 140ms; }
  .loading-list span:nth-child(3) { animation-delay: 280ms; }
  @keyframes directory-pulse { to { opacity: 0.25; transform: translateY(-3px); } }
  @media (max-width: 850px) {
    main { grid-template-columns: 1fr; }
    aside { position: absolute; inset: 0 auto 0 0; width: min(280px, 82vw); transform: translateX(-103%); box-shadow: var(--shadow-hover); transition: transform 180ms ease; }
    main.folders-open aside { transform: translateX(0); }
    .mobile-scrim { position: absolute; inset: 0; z-index: 5; display: block; border-radius: inherit; background: color-mix(in srgb, var(--text-main) 14%, transparent); opacity: 0; pointer-events: none; transition: opacity 180ms ease; }
    main.folders-open .mobile-scrim { opacity: 1; pointer-events: auto; }
    .mobile-folder-button { display: grid !important; }
  }
  @media (max-width: 650px) {
    .content { padding: 1rem; }
    .content-head h2 { max-width: 180px; }
  }
</style>
