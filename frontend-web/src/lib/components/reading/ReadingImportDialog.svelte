<script lang="ts">
  import { ApiError } from "$lib/api";
  import {
    createReadingArticle,
    getReadingArticle,
    saveReadingArticle,
    type ReadingArticleDetail,
    type ReadingErrorPayload,
  } from "$lib/readingApi";

  type FolderOption = { id: number; name: string };

  let { folders, defaultFolderId, articleId = null, onClose, onCreated } = $props<{
    folders: FolderOption[];
    defaultFolderId: number | null;
    articleId?: number | null;
    onClose: () => void;
    onCreated: (article: ReadingArticleDetail) => void;
  }>();

  let title = $state("");
  let body = $state("");
  let folderId = $state<number | null>(null);
  let saving = $state(false);
  let loading = $state(false);
  let error = $state<ReadingErrorPayload | null>(null);

  $effect(() => {
    const id = articleId;
    if (id) void loadArticle(id);
    else folderId = defaultFolderId;
  });

  async function loadArticle(id: number) {
    loading = true;
    error = null;
    try {
      const article = await getReadingArticle(id);
      title = article.title;
      body = article.body;
      folderId = article.folder_id;
    } catch (cause) {
      error = { message: cause instanceof Error ? cause.message : "文章加载失败" };
    } finally {
      loading = false;
    }
  }

  async function submit() {
    saving = true;
    error = null;
    try {
      const payload = { title, body, folder_id: folderId };
      onCreated(articleId ? await saveReadingArticle(articleId, payload) : await createReadingArticle(payload));
    } catch (cause) {
      if (cause instanceof ApiError && cause.payload && typeof cause.payload === "object") {
        error = cause.payload as ReadingErrorPayload;
      } else {
        error = { message: cause instanceof Error ? cause.message : "文章保存失败" };
      }
    } finally {
      saving = false;
    }
  }
</script>

<div class="dialog-backdrop" role="presentation" onclick={(event) => event.target === event.currentTarget && onClose()}>
  <div class="import-dialog" role="dialog" aria-modal="true" aria-labelledby="import-title">
    <header>
      <h2 id="import-title">{articleId ? "编辑文章" : "导入文章"}</h2>
      <button type="button" aria-label="关闭" onclick={onClose}><i class="ph ph-x"></i></button>
    </header>

    <form onsubmit={(event) => { event.preventDefault(); void submit(); }}>
      <label><span>标题</span><input bind:value={title} required /></label>
      <label>
        <span>文件夹</span>
        <select bind:value={folderId}>
          <option value={null}>未归类</option>
          {#each folders as folder (folder.id)}
            <option value={folder.id}>{folder.name}</option>
          {/each}
        </select>
      </label>
      <label class="body-field">
        <span>正文</span>
        <textarea bind:value={body} required placeholder="可直接粘贴正文；书签写作 [Reisen](Reise)"></textarea>
      </label>

      {#if error}
        <div class="save-error" role="alert">
          <strong>{error.message}</strong>
          {#if error.bookmarks?.length}
            <ul>
              {#each error.bookmarks as issue}
                <li><code>[{issue.surface}]({issue.form_of})</code><span>{issue.reason}</span></li>
              {/each}
            </ul>
          {/if}
        </div>
      {/if}

      <footer>
        <button type="button" class="secondary" onclick={onClose}>取消</button>
        <button type="submit" class="primary" disabled={saving || loading}>{loading ? "加载中…" : saving ? "检查中…" : "保存并打开"}</button>
      </footer>
    </form>
  </div>
</div>

<style>
  .dialog-backdrop { position: fixed; inset: 0; z-index: 600; display: grid; place-items: center; padding: 1rem; background: color-mix(in srgb, var(--text-main) 20%, transparent); backdrop-filter: blur(5px); }
  .import-dialog { width: min(680px, 100%); overflow: hidden; border: 1px solid var(--border-color); border-radius: var(--radius-lg); background: var(--card-bg); color: var(--text-main); box-shadow: var(--shadow-hover); }
  header { display: flex; min-height: 62px; align-items: center; justify-content: space-between; padding: 0 1.25rem; border-bottom: 1px solid var(--border-color); }
  h2 { margin: 0; font-size: 1.05rem; }
  header button { display: grid; width: 34px; height: 34px; place-items: center; border-radius: 50%; background: var(--btn-secondary); color: var(--text-main); }
  form { display: grid; grid-template-columns: minmax(0, 1fr) 190px; gap: 1rem; padding: 1.25rem; }
  label { display: grid; gap: 0.45rem; }
  label > span { color: var(--text-muted); font-size: 0.72rem; font-weight: 800; }
  input, select, textarea { width: 100%; border: 1px solid var(--border-color); border-radius: 10px; outline: 0; background: var(--bg-color); color: var(--text-main); font: inherit; }
  input, select { height: 42px; padding: 0 0.75rem; }
  textarea { min-height: 300px; resize: vertical; padding: 0.85rem; line-height: 1.65; }
  input:focus, select:focus, textarea:focus { border-color: color-mix(in srgb, var(--accent-main) 60%, var(--border-color)); box-shadow: 0 0 0 3px color-mix(in srgb, var(--accent-main) 10%, transparent); }
  .body-field, .save-error, footer { grid-column: 1 / -1; }
  .save-error { padding: 0.8rem 0.9rem; border: 1px solid color-mix(in srgb, #c64b43 28%, var(--border-color)); border-radius: 10px; background: color-mix(in srgb, #c64b43 7%, var(--card-bg)); color: var(--text-main); font-size: 0.76rem; }
  .save-error ul { display: grid; gap: 0.55rem; margin: 0.65rem 0 0; padding: 0; list-style: none; }
  .save-error li { display: grid; gap: 0.15rem; }
  .save-error code { color: var(--accent-main); }
  .save-error span { color: var(--text-muted); }
  footer { display: flex; justify-content: flex-end; gap: 0.6rem; padding-top: 0.15rem; }
  footer button { min-height: 40px; padding: 0 1rem; border-radius: var(--radius-full); font-weight: 800; }
  .secondary { background: var(--btn-secondary); color: var(--text-main); }
  .primary { background: var(--accent-main); color: white; }
  .primary:disabled { opacity: 0.55; }
  @media (max-width: 620px) {
    .dialog-backdrop { align-items: end; padding: 0; }
    .import-dialog { max-height: 94dvh; overflow-y: auto; border-radius: var(--radius-lg) var(--radius-lg) 0 0; }
    form { grid-template-columns: 1fr; padding: 1rem; }
    .body-field, .save-error, footer { grid-column: auto; }
    textarea { min-height: 42dvh; }
  }
</style>
