<script lang="ts">
  import ReadingLookupRail from "$lib/components/reading/ReadingLookupRail.svelte";

  const bookmarks = [
    { key: "Pauschalreise", label: "Pauschalreise", source: "Pauschalreise" },
    { key: "Reise", label: "Reise", source: "Reisen" },
    { key: "vorhaben", label: "vorhaben", source: "habe" },
    { key: "übernachten", label: "übernachten", source: "übernachten" },
    { key: "erholen", label: "erholen", source: "erholen" },
    { key: "Alltagsstress", label: "Alltagsstress", source: "Alltagsstress" },
  ];

  let selectedKey = $state("Pauschalreise");
  let showBookmarks = $state(false);
  let railOpen = $state(false);

  function selectWord(key: string) {
    selectedKey = key;
    railOpen = true;
  }

  function jumpToBookmark(bookmark: (typeof bookmarks)[number]) {
    selectWord(bookmark.key);
    showBookmarks = false;
    document.getElementById(`mark-${bookmark.key}`)?.scrollIntoView({ behavior: "smooth", block: "center" });
  }
</script>

<div class="reading-lab">
  <header class="page-head">
    <div class="title-group">
      <a class="back-button" href="/" aria-label="返回"><i class="ph ph-arrow-left"></i></a>
      <div><span>Netzwerk neu B1 · Reisen</span><h1>Reisen auf meine Art</h1></div>
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
                <strong>{bookmark.label}</strong><span>原文：{bookmark.source}</span>
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
      <article>
        <p>
          Obwohl eine
          <button id="mark-Pauschalreise" class:current={selectedKey === "Pauschalreise"} class="marked" onclick={() => selectWord("Pauschalreise")}>Pauschalreise</button>
          sehr bequem ist, organisiere ich meine
          <button id="mark-Reise" class:current={selectedKey === "Reise"} class="marked" onclick={() => selectWord("Reise")}>Reisen</button>
          lieber selbst.
        </p>
        <p>
          Da ich die Natur liebe,
          <button id="mark-vorhaben" class:current={selectedKey === "vorhaben"} class="marked" onclick={() => selectWord("vorhaben")}>habe</button>
          ich vor, diesen Sommer im Zelt zu
          <button id="mark-übernachten" class:current={selectedKey === "übernachten"} class="marked" onclick={() => selectWord("übernachten")}>übernachten</button>{"."}
        </p>
        <p>
          Es ist fantastisch, sich in den Bergen zu
          <button id="mark-erholen" class:current={selectedKey === "erholen"} class="marked" onclick={() => selectWord("erholen")}>erholen</button>
          und den
          <button id="mark-Alltagsstress" class:current={selectedKey === "Alltagsstress"} class="marked" onclick={() => selectWord("Alltagsstress")}>Alltagsstress</button>
          zu vergessen.
        </p>
      </article>

      <footer class="article-foot">
        <span>40 Wörter</span><span>6 标记</span>
      </footer>
    </main>

    <ReadingLookupRail selectedKey={selectedKey} open={railOpen} onClose={() => railOpen = false} />
  </div>
</div>

<style>
  :global(.page-container:has(.reading-lab)) { max-width: 1320px; }
  .reading-lab { position: relative; display: grid; height: calc(100dvh - 4rem); grid-template-rows: auto minmax(0, 1fr); gap: 1rem; color: var(--text-main); }
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
  article { width: min(720px, calc(100% - 5rem)); margin: auto; padding: clamp(4rem, 10vh, 7rem) 0; }
  article p { margin: 0; font-family: "Avenir Next", Avenir, "Segoe UI", sans-serif; font-size: clamp(1.3rem, 1.65vw, 1.6rem); font-weight: 540; line-height: 2; letter-spacing: 0.005em; }
  article p + p { margin-top: 1.8rem; }
  .marked { position: relative; display: inline; padding: 0 0.04em 0.04em; border-radius: 0; background: transparent; color: inherit; font: inherit; font-weight: 690; transition: background 120ms ease, text-decoration-color 120ms ease; }
  .marked { box-shadow: inset 0 -2px color-mix(in srgb, var(--accent-main) 58%, transparent); }
  .marked::after { content: ""; position: absolute; left: calc(100% + 3px); bottom: -1px; width: 5px; height: 5px; border-radius: 50%; background: color-mix(in srgb, var(--accent-main) 72%, transparent); transition: background 120ms ease, transform 120ms ease; }
  .marked:hover, .marked.current { box-shadow: inset 0 -3px var(--accent-main); }
  .marked:hover::after, .marked.current::after { background: var(--accent-main); transform: scale(1.16); }
  .article-foot { display: flex; justify-content: space-between; margin: 0 2rem; padding: 1rem 0; border-top: 1px solid var(--border-color); color: var(--text-muted); font-size: 0.72rem; font-weight: 750; }

  @media (max-width: 900px) {
    :global(.page-container:has(.reading-lab)) { max-width: 100%; }
    .reading-lab { height: auto; min-height: calc(100dvh - 4rem); }
    .page-head { padding-right: 82px; }
    .head-actions { position: static; }
    .reading-shell { display: block; min-height: calc(100dvh - 8.8rem); }
    .article-pane { min-height: calc(100dvh - 9rem); }
    article { width: min(680px, calc(100% - 2.5rem)); padding: 3.5rem 0; }
    .mobile-lookup { display: flex; }
  }

  @media (max-width: 600px) {
    .reading-lab { min-height: calc(100dvh - 4rem); gap: 0.6rem; }
    .page-head { min-height: 66px; align-items: center; padding: 0.55rem 0.65rem; }
    .back-button { display: none; }
    .title-group span { font-size: 0.64rem; }
    h1 { font-size: 1.15rem; }
    .bookmark-menu > button { padding: 0 0.65rem; font-size: 0.78rem; }
    .bookmark-menu > button i { display: none; }
    .mobile-lookup { width: 40px; padding: 0; justify-content: center; gap: 0; color: transparent; font-size: 0; }
    .mobile-lookup i { color: var(--text-main); font-size: 1rem; }
    .reading-shell { border-radius: var(--radius-md); }
    article { width: calc(100% - 2rem); }
    article p { font-size: 1.2rem; line-height: 1.95; }
    .article-foot { margin: 0 1rem; }
  }
</style>
