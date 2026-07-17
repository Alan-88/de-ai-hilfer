<script lang="ts">
  type ContextTarget = {
    kind: "folder" | "article";
    id: number;
    x: number;
    y: number;
  };

  interface Props {
    target: ContextTarget | null;
    onRename: () => void;
    onEdit: () => void;
    onToggleStatus: () => void;
    statusLabel: string;
    onDelete: () => void;
    onCreateChild: () => void;
  }

  let { target, onRename, onEdit, onToggleStatus, statusLabel, onDelete, onCreateChild }: Props = $props();
</script>

{#if target}
  <div
    class="directory-context-menu"
    role="menu"
    aria-label={target.kind === "folder" ? "文件夹操作" : "文章操作"}
    style:left={`${target.x}px`}
    style:top={`${target.y}px`}
  >
    {#if target.kind === "folder"}
      <button role="menuitem" onclick={onCreateChild}><i class="ph ph-folder-plus"></i> 新建子文件夹</button>
    {:else}
      <button role="menuitem" onclick={onEdit}><i class="ph ph-note-pencil"></i> 编辑文章</button>
      <button role="menuitem" onclick={onToggleStatus}><i class="ph ph-check-circle"></i> {statusLabel}</button>
    {/if}
    <button role="menuitem" onclick={onRename}><i class="ph ph-pencil-simple"></i> 重命名</button>
    <div class="separator"></div>
    <button class="danger" role="menuitem" onclick={onDelete}><i class="ph ph-trash"></i> 删除</button>
  </div>
{/if}

<style>
  .directory-context-menu { position: fixed; z-index: 500; display: grid; width: 176px; padding: 0.38rem; border: 1px solid var(--border-color); border-radius: 11px; background: color-mix(in srgb, var(--card-bg) 96%, transparent); box-shadow: var(--shadow-hover); backdrop-filter: blur(14px); }
  button { display: flex; min-height: 36px; align-items: center; gap: 0.6rem; padding: 0 0.65rem; border-radius: 7px; background: transparent; color: var(--text-main); font-size: 0.76rem; font-weight: 700; text-align: left; }
  button:hover, button:focus-visible { background: var(--btn-secondary); outline: none; }
  button i { width: 16px; color: var(--text-muted); font-size: 0.9rem; }
  button.danger { color: var(--danger-text, #b8453f); }
  button.danger i { color: currentColor; }
  .separator { height: 1px; margin: 0.28rem 0.4rem; background: var(--border-color); }
</style>
