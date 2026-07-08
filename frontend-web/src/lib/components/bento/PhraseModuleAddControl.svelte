<script lang="ts">
  import { fade } from "svelte/transition";

  let {
    disabled = false,
    isLoading = false,
    onSubmit,
  } = $props<{
    disabled?: boolean;
    isLoading?: boolean;
    onSubmit?: (phrase: string) => Promise<void> | void;
  }>();

  let isOpen = $state(false);
  let phrase = $state("");
  let error = $state("");
  let inputRef = $state<HTMLInputElement | null>(null);

  $effect(() => {
    if (isOpen && inputRef) {
      requestAnimationFrame(() => inputRef?.focus());
    }
  });

  function toggle() {
    isOpen = !isOpen;
    error = "";
    if (!isOpen) phrase = "";
  }

  async function submit() {
    const value = phrase.trim();
    if (!value || disabled || isLoading || !onSubmit) return;

    error = "";
    try {
      await onSubmit(value);
      phrase = "";
      isOpen = false;
    } catch {
      error = "短语卡片添加失败";
    }
  }

  function handleKeydown(event: KeyboardEvent) {
    if (event.key === "Enter") {
      event.preventDefault();
      void submit();
    }
    if (event.key === "Escape") {
      isOpen = false;
      phrase = "";
      error = "";
    }
  }
</script>

<div class="phrase-module-control">
  <div class="phrase-module-add" class:is-open={isOpen}>
    <form
      class="phrase-module-search"
      onsubmit={(event) => {
        event.preventDefault();
        void submit();
      }}
      aria-hidden={!isOpen}
    >
      <button
        class="phrase-search-icon"
        type="submit"
        disabled={!phrase.trim() || disabled || isLoading}
        tabindex={isOpen ? 0 : -1}
        aria-label="添加短语卡片"
      >
        {#if isLoading}
          <span class="phrase-spinner"></span>
        {:else}
          <svg viewBox="0 0 24 24" aria-hidden="true">
            <path d="M10.8 18.1a7.3 7.3 0 1 1 5.16-12.46 7.3 7.3 0 0 1-5.16 12.46Zm0-2a5.3 5.3 0 1 0 0-10.6 5.3 5.3 0 0 0 0 10.6Z" />
            <path d="M16.2 15.1 21 19.9l-1.4 1.4-4.8-4.8 1.4-1.4Z" />
          </svg>
        {/if}
      </button>
      <input
        bind:this={inputRef}
        bind:value={phrase}
        onkeydown={handleKeydown}
        placeholder="搜索或输入要挂载的短语..."
        disabled={disabled || isLoading}
        autocomplete="off"
        tabindex={isOpen ? 0 : -1}
      />
    </form>
    <button
      class="phrase-add-toggle"
      class:is-open={isOpen}
      type="button"
      onclick={toggle}
      disabled={disabled || isLoading}
      aria-label={isOpen ? "关闭短语添加" : "添加短语卡片"}
      title={isOpen ? "关闭" : "添加短语卡片"}
    >
      <svg viewBox="0 0 24 24" aria-hidden="true">
        <path d="M11 5h2v14h-2z" />
        <path d="M5 11h14v2H5z" />
      </svg>
    </button>
  </div>
  {#if error}
    <div class="message-surface error compact-error" transition:fade>{error}</div>
  {/if}
</div>

<style>
  .phrase-module-control {
    display: flex;
    flex-direction: column;
    align-items: flex-end;
    gap: 0.45rem;
    min-width: 0;
  }

  .phrase-module-add {
    display: flex;
    align-items: center;
    justify-content: flex-end;
    gap: 0.45rem;
    min-height: 2.35rem;
  }

  .phrase-module-search {
    width: 0;
    max-width: min(360px, 54vw);
    opacity: 0;
    overflow: hidden;
    pointer-events: none;
    display: flex;
    align-items: center;
    gap: 0.45rem;
    border: 1px solid var(--border-color);
    border-radius: 999px;
    background: var(--bg-color);
    padding: 0;
    transition: width 0.28s ease, opacity 0.18s ease, padding 0.28s ease;
  }

  .phrase-module-add.is-open .phrase-module-search {
    width: min(360px, 54vw);
    opacity: 1;
    pointer-events: auto;
    padding: 0.25rem 0.75rem 0.25rem 0.45rem;
  }

  .phrase-search-icon,
  .phrase-add-toggle {
    width: 2rem;
    height: 2rem;
    border-radius: 999px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    color: var(--text-muted);
    flex: 0 0 auto;
  }

  .phrase-search-icon {
    background: transparent;
  }

  .phrase-search-icon:hover:not(:disabled) {
    color: var(--accent-main);
  }

  .phrase-add-toggle {
    background: var(--btn-secondary);
    color: var(--text-main);
    transition: transform 0.24s ease, background 0.2s ease, color 0.2s ease;
  }

  .phrase-add-toggle.is-open {
    transform: rotate(45deg);
    background: var(--text-main);
    color: var(--bg-color);
  }

  .phrase-search-icon svg,
  .phrase-add-toggle svg {
    width: 1.05rem;
    height: 1.05rem;
    fill: currentColor;
  }

  .phrase-module-search input {
    width: 100%;
    min-width: 0;
    height: 1.9rem;
    background: transparent;
    color: var(--text-main);
    font-size: 0.9rem;
  }

  .phrase-spinner {
    width: 0.9rem;
    height: 0.9rem;
    border-radius: 999px;
    border: 2px solid color-mix(in srgb, var(--accent-main) 24%, transparent);
    border-top-color: var(--accent-main);
    animation: spin 0.75s linear infinite;
  }

  .compact-error {
    padding: 0.75rem 0.9rem;
  }

  @media (max-width: 900px) {
    .phrase-module-control,
    .phrase-module-add,
    .phrase-module-add.is-open .phrase-module-search {
      width: 100%;
    }

    .phrase-module-search,
    .phrase-module-add.is-open .phrase-module-search {
      max-width: none;
    }
  }
</style>
