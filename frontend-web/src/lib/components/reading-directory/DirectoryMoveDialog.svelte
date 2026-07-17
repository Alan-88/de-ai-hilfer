<script lang="ts">
  interface FolderOption {
    id: number;
    name: string;
  }

  interface Props {
    articleTitle: string;
    folders: FolderOption[];
    currentFolderId: number | null;
    moving: boolean;
    onSelect: (folderId: number | null) => void;
    onClose: () => void;
  }

  let { articleTitle, folders, currentFolderId, moving, onSelect, onClose }: Props = $props();
</script>

<svelte:window onkeydown={(event) => { if (event.key === "Escape" && !moving) onClose(); }} />

<div class="move-overlay">
  <button class="move-scrim" aria-label="关闭移动面板" disabled={moving} onclick={onClose}></button>
  <div class="move-dialog" role="dialog" tabindex="-1" aria-modal="true" aria-labelledby="move-title">
    <header>
      <div><h2 id="move-title">移动文章</h2><span>{articleTitle}</span></div>
      <button aria-label="关闭" disabled={moving} onclick={onClose}><i class="ph ph-x"></i></button>
    </header>
    <div class="folder-options">
      <button class:current={currentFolderId === null} disabled={moving || currentFolderId === null} onclick={() => onSelect(null)}>
        <i class="ph ph-tray"></i><span>未归类</span>{#if currentFolderId === null}<i class="ph ph-check"></i>{/if}
      </button>
      {#each folders as folder}
        <button class:current={currentFolderId === folder.id} disabled={moving || currentFolderId === folder.id} onclick={() => onSelect(folder.id)}>
          <i class="ph ph-folder"></i><span>{folder.name}</span>{#if currentFolderId === folder.id}<i class="ph ph-check"></i>{/if}
        </button>
      {/each}
    </div>
  </div>
</div>

<style>
  .move-overlay { position: fixed; inset: 0; z-index: 560; display: grid; place-items: center; padding: 1rem; }
  .move-scrim { position: absolute; inset: 0; background: color-mix(in srgb, var(--text-main) 16%, transparent); backdrop-filter: blur(2px); }
  .move-dialog { position: relative; width: min(390px, calc(100vw - 2rem)); overflow: hidden; border: 1px solid var(--border-color); border-radius: var(--radius-lg); background: var(--card-bg); box-shadow: var(--shadow-hover); color: var(--text-main); }
  header { display: flex; min-height: 68px; align-items: center; justify-content: space-between; gap: 1rem; padding: 0.8rem 0.85rem 0.8rem 1rem; border-bottom: 1px solid var(--border-color); }
  header div { display: grid; min-width: 0; gap: 0.18rem; }
  h2 { margin: 0; font-size: 0.98rem; }
  header span { overflow: hidden; color: var(--text-muted); font-size: 0.7rem; text-overflow: ellipsis; white-space: nowrap; }
  header button { display: grid; width: 32px; height: 32px; flex: 0 0 auto; place-items: center; border-radius: 50%; background: var(--btn-secondary); color: var(--text-main); }
  .folder-options { display: grid; max-height: min(420px, 56vh); overflow-y: auto; padding: 0.45rem; }
  .folder-options button { display: grid; min-height: 44px; grid-template-columns: 20px minmax(0, 1fr) 20px; align-items: center; gap: 0.55rem; padding: 0 0.7rem; border-radius: 9px; background: transparent; color: var(--text-main); text-align: left; }
  .folder-options button:hover:not(:disabled), .folder-options button:focus-visible { background: var(--btn-secondary); outline: none; }
  .folder-options button.current { color: var(--accent-main); }
  .folder-options button:disabled { opacity: 1; }
  .folder-options button > i:first-child { color: var(--text-muted); }
  .folder-options button.current > i { color: currentColor; }
  .folder-options span { overflow: hidden; font-size: 0.78rem; font-weight: 750; text-overflow: ellipsis; white-space: nowrap; }
  @media (max-width: 650px) {
    .move-overlay { align-items: end; padding: 0; }
    .move-dialog { width: 100%; border-width: 1px 0 0; border-radius: var(--radius-lg) var(--radius-lg) 0 0; padding-bottom: env(safe-area-inset-bottom); }
  }
</style>
