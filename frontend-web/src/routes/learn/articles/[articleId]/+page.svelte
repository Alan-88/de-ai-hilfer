<script lang="ts">
  import { page } from "$app/state";
  import ReadingArticleView from "$lib/components/reading/ReadingArticleView.svelte";

  const articleId = $derived(Number(page.params.articleId));
</script>

<svelte:head>
  <title>文章阅读 | De-AI-Hilfer</title>
</svelte:head>

<ReadingArticleView {articleId} />
