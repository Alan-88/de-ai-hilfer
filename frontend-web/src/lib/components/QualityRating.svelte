<script lang="ts">
  import { ReviewQuality } from "$lib/types";

  export let disabled = false;
  export let onRating: (quality: ReviewQuality) => void = () => {};

  const options = [
    { value: ReviewQuality.COMPLETELY_FORGOT, label: "完全忘记" },
    { value: ReviewQuality.INCORRECT_BUT_REMEMBERED, label: "错误但有印象" },
    { value: ReviewQuality.INCORRECT_WITH_HINT, label: "提示后记起" },
    { value: ReviewQuality.HESITANT, label: "犹豫但正确" },
    { value: ReviewQuality.CORRECT_WITH_HESITATION, label: "正确但犹豫" },
    { value: ReviewQuality.PERFECT, label: "完美回忆" }
  ];
</script>

<div class="rating-panel">
  <p>这次回忆质量如何？</p>
  <div class="grid">
    {#each options as option}
      <button disabled={disabled} on:click={() => onRating(option.value)}>
        {option.label}
      </button>
    {/each}
  </div>
</div>

<style>
  .rating-panel p {
    margin: 0 0 0.8rem;
    color: var(--muted);
    font-family: var(--font-ui);
  }

  .grid {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 0.65rem;
  }

  button {
    border: 1px solid var(--border);
    background: #fff;
    border-radius: 0.9rem;
    padding: 0.8rem 0.9rem;
    color: var(--text);
  }

  button:disabled {
    opacity: 0.55;
    cursor: not-allowed;
  }
</style>
