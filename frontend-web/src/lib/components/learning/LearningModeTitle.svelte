<script lang="ts">
  import { page } from "$app/state";

  const articleMode = $derived(
    page.url.pathname.startsWith("/learn/articles")
  );
</script>

<header class="mode-header">
  <h1 class="accessible-title">{articleMode ? "文章学习" : "单词背诵"}</h1>
  <nav class:review-active={!articleMode} aria-label="学习模式">
    <a href="/learn/articles" class:active={articleMode} aria-current={articleMode ? "page" : undefined}>
      <span>文章学习</span>
    </a>
    <a href="/learn" class:active={!articleMode} aria-current={!articleMode ? "page" : undefined}>
      <span>单词背诵</span>
    </a>
    <i aria-hidden="true"></i>
  </nav>
</header>

<style>
  .mode-header { display: flex; min-height: 70px; align-items: flex-end; padding-right: 88px; }
  .accessible-title { position: absolute; width: 1px; height: 1px; overflow: hidden; clip: rect(0 0 0 0); white-space: nowrap; }
  nav { --slot: 104px; --indicator: 56px; position: relative; display: grid; width: calc(var(--slot) * 2); height: 66px; grid-template-columns: repeat(2, var(--slot)); }
  a { display: flex; height: 60px; align-items: flex-end; justify-content: center; padding-bottom: 16px; color: var(--text-muted); text-decoration: none; }
  a span { display: block; font-size: 1rem; font-weight: 720; line-height: 1; transform-origin: center bottom; transition: color 190ms ease, transform 210ms cubic-bezier(0.22, 1, 0.36, 1); }
  a.active { color: var(--text-main); }
  a.active span { font-weight: 860; transform: scale(1.58); }
  a:not(.active):hover span { color: var(--text-main); transform: translateY(-1px); }
  a:focus-visible { border-radius: 8px; outline: 2px solid color-mix(in srgb, var(--accent-main) 55%, transparent); outline-offset: -5px; }
  nav > i { position: absolute; bottom: 5px; left: calc((var(--slot) - var(--indicator)) / 2); width: var(--indicator); height: 3px; border-radius: var(--radius-full); background: var(--accent-main); box-shadow: 0 2px 7px color-mix(in srgb, var(--accent-main) 24%, transparent); transition: transform 220ms cubic-bezier(0.22, 1, 0.36, 1); }
  nav.review-active > i { transform: translateX(var(--slot)); }

  @media (max-width: 650px) {
    .mode-header { min-height: 70px; padding-right: 82px; }
    nav { --slot: 84px; --indicator: 46px; height: 64px; }
    a { height: 58px; padding-bottom: 15px; }
    a span { font-size: 0.92rem; }
    a.active span { transform: scale(1.3); }
  }

  @media (prefers-reduced-motion: reduce) {
    a span, nav > i { transition-duration: 0.01ms; }
  }
</style>
