<script lang="ts">
  import "../app.css";
  import { page } from "$app/state";
  import { theme } from "$lib/stores/ui";
  import { onMount } from "svelte";

  let { children } = $props();

  const tabs = [
    {
      id: "search",
      path: "/",
      icon: "ph ph-magnifying-glass",
      activeIcon: "ph-fill ph-magnifying-glass",
      label: "查词探索",
      mobileLabel: "查词",
    },
    {
      id: "library",
      path: "/library",
      icon: "ph ph-books",
      activeIcon: "ph ph-books",
      label: "知识词库",
      mobileLabel: "词库",
    },
    {
      id: "learn",
      path: "/learn",
      icon: "ph ph-target",
      activeIcon: "ph ph-target",
      label: "沉浸复习",
      mobileLabel: "复习",
    },
    {
      id: "settings",
      path: "/manage",
      icon: "ph ph-gear",
      activeIcon: "ph ph-gear",
      label: "数据设置",
      mobileLabel: "设置",
    },
  ];

  function isActive(path: string) {
    if (path === "/") return page.url.pathname === "/";
    return page.url.pathname.startsWith(path);
  }

  function setTheme(nextTheme: 'canvas' | 'canvas-dark') {
    $theme = nextTheme;
  }
</script>

<div class="app-layout" id="app">
  <aside class="sidebar">
    <div class="logo"><i class="ph-fill ph-translate"></i> De-AI-Hilfer</div>
    <nav class="nav-menu">
      {#each tabs as tab}
        <a
          href={tab.path}
          class="nav-item"
          class:active={isActive(tab.path)}
        >
          <i class={isActive(tab.path) ? tab.activeIcon : tab.icon}></i> {tab.label}
        </a>
      {/each}
    </nav>
  </aside>

  <nav class="bottom-nav">
    <div class="nav-menu">
      {#each tabs as tab}
        <a
          href={tab.path}
          class="nav-item"
          class:active={isActive(tab.path)}
        >
          <i class={isActive(tab.path) ? tab.activeIcon : tab.icon}></i><span>{tab.mobileLabel}</span>
        </a>
      {/each}
    </div>
  </nav>

  <main class="main-area">
    <div class="theme-switcher-group">
      <div class="theme-switch-track">
        <div class="active-slider" class:dark={$theme === 'canvas-dark'}></div>
        <button
          class="theme-btn"
          class:active={$theme === "canvas"}
          title="切换浅色模式"
          onclick={() => setTheme("canvas")}
        >
          <i class="ph-fill ph-sun"></i>
        </button>
        <button
          class="theme-btn"
          class:active={$theme === "canvas-dark"}
          title="切换深色模式"
          onclick={() => setTheme("canvas-dark")}
        >
          <i class="ph-fill ph-moon"></i>
        </button>
      </div>
    </div>

    <div class="page-container">
      {@render children()}
    </div>
  </main>
</div>

<style>
  /* 布局核心样式，确保不会因为 fixed 导致竞态或溢出 */
  .app-layout {
    display: flex;
    width: 100%;
    height: 100vh;
    overflow: hidden;
  }

  .main-area {
    flex: 1;
    position: relative;
    overflow-y: auto;
    padding: 2rem;
    display: flex;
    flex-direction: column;
    align-items: center;
  }

  /* 升级版主题切换器样式 */
  .theme-switcher-group {
    position: absolute;
    top: 1.5rem;
    right: 1.5rem;
    z-index: 100;
  }

  .theme-switch-track {
    background: var(--btn-secondary);
    border: 1px solid var(--border-color);
    padding: 0.3rem;
    border-radius: var(--radius-full);
    display: flex;
    position: relative;
    box-shadow: var(--shadow-sm);
    gap: 0.2rem;
  }

  .active-slider {
    position: absolute;
    top: 0.3rem;
    left: 0.3rem;
    width: 32px;
    height: 32px;
    background: var(--card-bg);
    border-radius: 50%;
    box-shadow: 0 4px 12px rgba(0,0,0,0.1), 0 0 0 1px var(--border-color);
    transition: transform 0.5s cubic-bezier(0.34, 1.56, 0.64, 1);
    z-index: 1;
  }

  /* 模拟 Dribbble 环境光 */
  .active-slider::after {
    content: '';
    position: absolute;
    inset: -4px;
    border-radius: 50%;
    background: var(--accent-glow);
    filter: blur(8px);
    opacity: 0.4;
    z-index: -1;
  }

  .active-slider.dark {
    transform: translateX(34px);
  }

  .theme-btn {
    width: 32px;
    height: 32px;
    border-radius: 50%;
    display: flex;
    align-items: center;
    justify-content: center;
    background: transparent !important; /* 强制背景透明 */
    color: var(--text-muted);
    position: relative;
    z-index: 2;
    transition: all var(--transition-fast);
    padding: 0;
    margin: 0;
  }

  .theme-btn.active {
    color: var(--accent-main);
    transform: scale(1.1);
  }

  .theme-btn:hover:not(.active) {
    color: var(--text-main);
  }

  .page-container {
    width: 100%;
    max-width: 960px;
    animation: fadeIn 0.3s ease;
  }

  @keyframes fadeIn {
    from { opacity: 0; transform: translateY(10px); }
    to { opacity: 1; transform: translateY(0); }
  }

  /* 侧边栏/导航基础样式从 app.css 逐步迁移到这里 */
  .sidebar {
    width: 240px;
    background-color: var(--sidebar-bg);
    padding: 2rem 1.25rem;
    display: flex;
    flex-direction: column;
    border-right: 1px solid var(--border-color);
    z-index: 10;
  }

  .logo {
    font-size: 1.25rem;
    font-weight: 800;
    margin-bottom: 2.5rem;
    padding-left: 0.5rem;
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }

  .nav-menu {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .nav-item {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    padding: 0.8rem 1rem;
    border-radius: var(--radius-sm);
    color: var(--nav-text);
    text-decoration: none;
    font-weight: 600;
  }

  .nav-item.active {
    background-color: var(--btn-secondary);
    color: var(--accent-main);
  }

  .bottom-nav {
    display: none;
  }

  @media (max-width: 768px) {
    .sidebar { display: none; }
    .bottom-nav {
      display: block;
      position: fixed;
      bottom: 0;
      left: 0;
      right: 0;
      background-color: var(--sidebar-bg);
      border-top: 1px solid var(--border-color);
      padding: 0.5rem 1rem calc(0.5rem + env(safe-area-inset-bottom));
      z-index: 100;
    }
    .bottom-nav .nav-menu {
      flex-direction: row;
      justify-content: space-around;
    }
    .main-area {
      padding: 1.5rem 1rem calc(4rem + env(safe-area-inset-bottom));
    }
  }
</style>
