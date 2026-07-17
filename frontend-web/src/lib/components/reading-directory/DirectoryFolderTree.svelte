<script lang="ts">
  type Folder = { id: number; name: string; parentId: number | null };
  type DragItem = { kind: "folder" | "article"; id: number };
  type VisibleFolder = Folder & { depth: number; hasChildren: boolean };

  interface Props {
    folders: Folder[];
    expandedIds: number[];
    selectedFolderId: number | null;
    folderViewActive: boolean;
    dragItem: DragItem | null;
    countForFolder: (id: number) => number;
    onSelect: (id: number) => void;
    onToggle: (id: number) => void;
    onCreate: () => void;
    onDragStart: (event: DragEvent, item: DragItem) => void;
    onDragEnd: () => void;
    onDrop: (folderId: number | null) => void;
    onContextMenu: (event: MouseEvent, kind: "folder", id: number) => void;
    onPressStart: (event: PointerEvent, kind: "folder", id: number) => void;
    onPressMove: (event: PointerEvent) => void;
    onPressEnd: () => void;
  }

  let {
    folders, expandedIds, selectedFolderId, folderViewActive, dragItem, countForFolder,
    onSelect, onToggle, onCreate, onDragStart, onDragEnd, onDrop, onContextMenu,
    onPressStart, onPressMove, onPressEnd,
  }: Props = $props();

  let dropTarget = $state<number | "root" | null>(null);

  const visibleFolders = $derived.by(() => {
    const result: VisibleFolder[] = [];
    const append = (parentId: number | null, depth: number) => {
      for (const folder of folders.filter((item) => item.parentId === parentId)) {
        const hasChildren = folders.some((item) => item.parentId === folder.id);
        result.push({ ...folder, depth, hasChildren });
        if (hasChildren && expandedIds.includes(folder.id)) append(folder.id, depth + 1);
      }
    };
    append(null, 0);
    return result;
  });

  function allowDrop(event: DragEvent, target: number | "root") {
    if (!dragItem) return;
    event.preventDefault();
    if (event.dataTransfer) event.dataTransfer.dropEffect = "move";
    dropTarget = target;
  }

  function drop(event: DragEvent, folderId: number | null) {
    event.preventDefault();
    dropTarget = null;
    onDrop(folderId);
  }
</script>

<div class="folder-tree">
  <div
    class:drop-target={dropTarget === "root"}
    class="tree-heading"
    role="group"
    aria-label="根目录放置区"
    ondragover={(event) => allowDrop(event, "root")}
    ondragleave={() => dropTarget = null}
    ondrop={(event) => drop(event, null)}
  >
    <span>文件夹</span>
    <button aria-label="新建文件夹" title="新建文件夹" onclick={onCreate}><i class="ph ph-plus"></i></button>
  </div>

  <nav aria-label="文章文件夹">
    {#each visibleFolders as folder (folder.id)}
      <div
        class:active={folderViewActive && selectedFolderId === folder.id}
        class:drop-target={dropTarget === folder.id}
        class="folder-row"
        role="group"
        aria-label={folder.name}
        style:padding-left={`${0.35 + folder.depth * 0.82}rem`}
        oncontextmenu={(event) => onContextMenu(event, "folder", folder.id)}
        onpointerdown={(event) => onPressStart(event, "folder", folder.id)}
        onpointermove={onPressMove}
        onpointerup={onPressEnd}
        onpointercancel={onPressEnd}
        ondragover={(event) => allowDrop(event, folder.id)}
        ondragleave={() => dropTarget = null}
        ondrop={(event) => drop(event, folder.id)}
      >
        <button
          class:invisible={!folder.hasChildren}
          class="caret"
          aria-label={expandedIds.includes(folder.id) ? "折叠文件夹" : "展开文件夹"}
          onclick={() => onToggle(folder.id)}
        >
          <i class:expanded={expandedIds.includes(folder.id)} class="ph ph-caret-right"></i>
        </button>
        <button
          class="drag-handle"
          draggable="true"
          aria-label={`拖动文件夹 ${folder.name}`}
          title="拖动文件夹"
          ondragstart={(event) => onDragStart(event, { kind: "folder", id: folder.id })}
          ondragend={onDragEnd}
        ><i class="ph ph-folder"></i></button>
        <button class="folder-name" onclick={() => onSelect(folder.id)}>{folder.name}</button>
        <em>{countForFolder(folder.id)}</em>
      </div>
    {/each}
  </nav>
</div>

<style>
  .tree-heading { display: flex; min-height: 38px; align-items: center; justify-content: space-between; margin: 0.95rem 0.25rem 0.35rem; padding: 0 0.3rem; border-radius: 8px; color: var(--text-muted); transition: background 120ms ease, color 120ms ease; }
  .tree-heading > span { font-size: 0.64rem; font-weight: 850; letter-spacing: 0.08em; }
  .tree-heading > button { display: grid; width: 28px; height: 28px; place-items: center; border-radius: 50%; background: transparent; color: inherit; }
  nav { display: grid; gap: 0.14rem; }
  .folder-row { display: grid; min-height: 38px; grid-template-columns: 18px 24px minmax(0, 1fr) auto; align-items: center; gap: 0.18rem; padding-right: 0.5rem; border-radius: 8px; color: var(--text-muted); transition: background 120ms ease, box-shadow 120ms ease, color 120ms ease; }
  .folder-row:hover { background: color-mix(in srgb, var(--card-bg) 62%, transparent); color: var(--text-main); }
  .folder-row.active { background: var(--card-bg); color: var(--text-main); box-shadow: var(--shadow-sm); }
  .drop-target { background: color-mix(in srgb, var(--accent-main) 12%, var(--card-bg)); color: var(--text-main); box-shadow: inset 0 0 0 1px color-mix(in srgb, var(--accent-main) 38%, transparent); }
  .caret, .drag-handle { display: grid; width: 24px; height: 28px; place-items: center; border-radius: 6px; background: transparent; color: inherit; }
  .caret { width: 18px; }
  .caret.invisible { visibility: hidden; }
  .caret i { font-size: 0.66rem; transition: transform 130ms ease; }
  .caret i.expanded { transform: rotate(90deg); }
  .drag-handle { cursor: grab; }
  .drag-handle:active { cursor: grabbing; }
  .drag-handle:hover { background: var(--btn-secondary); color: var(--accent-main); }
  .folder-name { overflow: hidden; height: 100%; background: transparent; color: inherit; font-size: 0.76rem; font-weight: 750; text-align: left; text-overflow: ellipsis; white-space: nowrap; }
  em { font-size: 0.65rem; font-style: normal; font-variant-numeric: tabular-nums; }
</style>
