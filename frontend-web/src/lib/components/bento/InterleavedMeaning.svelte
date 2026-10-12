<script lang="ts">
  let { pos, chinese, english = "" } = $props<{ pos: string; chinese: string; english?: string }>();
  let root = $state<HTMLElement>();
  let label = $state<HTMLElement>();
  let chineseProbe = $state<HTMLElement>();
  let englishProbe = $state<HTMLElement>();
  let chineseLines = $state<string[]>([]);
  let englishLines = $state<string[]>([]);
  let indent = $state(0);

  function linesOf(element: HTMLElement): string[] {
    const text = element.firstChild;
    if (!text?.textContent) return [];
    const lines: { top: number; text: string }[] = [];
    const range = document.createRange();
    for (let index = 0; index < text.textContent.length; index++) {
      range.setStart(text, index);
      range.setEnd(text, index + 1);
      const top = range.getBoundingClientRect().top;
      const last = lines.at(-1);
      if (last && Math.abs(last.top - top) < 1) last.text += text.textContent[index];
      else lines.push({ top, text: text.textContent[index] });
    }
    return lines.map((line) => line.text);
  }

  $effect(() => {
    // Reflow on content, available width, or loaded font changes.
    pos; chinese; english;
    if (!root || !label || !chineseProbe || !englishProbe) return;
    const container = root;
    const tag = label;
    const zh = chineseProbe;
    const en = englishProbe;
    let frame = 0;
    let disposed = false;
    let width = -1;
    const measure = () => {
      if (disposed) return;
      indent = Math.min(tag.getBoundingClientRect().width + 8, container.clientWidth);
      zh.style.textIndent = en.style.textIndent = `${indent}px`;
      chineseLines = linesOf(zh);
      englishLines = linesOf(en);
    };
    const schedule = () => {
      cancelAnimationFrame(frame);
      frame = requestAnimationFrame(measure);
    };
    const observer = new ResizeObserver(([entry]) => {
      if (entry.contentRect.width !== width) {
        width = entry.contentRect.width;
        schedule();
      }
    });
    observer.observe(container);
    document.fonts.addEventListener("loadingdone", schedule);
    void document.fonts.ready.then(() => { if (!disposed) schedule(); });
    schedule();
    return () => {
      disposed = true;
      cancelAnimationFrame(frame);
      observer.disconnect();
      document.fonts.removeEventListener("loadingdone", schedule);
    };
  });
</script>

<section class="interleaved-meaning" bind:this={root} style:--meaning-indent={`${indent}px`}>
  <span class="accessible-text">{pos} {chinese} {english}</span>
  <div class="visual-lines" aria-hidden="true">
    <span class="pos-label" class:bilingual={Boolean(chinese && english)} bind:this={label}>{pos}</span>
    {#each Array.from({ length: Math.max(chineseLines.length, englishLines.length) }) as _, index}
      <div class="line-pair" class:first={index === 0}>
        {#if chineseLines[index]}<p class="chinese">{chineseLines[index]}</p>{/if}
        {#if englishLines[index]}<p class="english">{englishLines[index]}</p>{/if}
      </div>
    {/each}
  </div>
  <p class="probe chinese" aria-hidden="true" bind:this={chineseProbe}>{chinese}</p>
  <p class="probe english" aria-hidden="true" bind:this={englishProbe}>{english}</p>
</section>

<style>
  .interleaved-meaning { position: relative; min-width: 0; text-align: left; }
  .visual-lines { position: relative; }
  .pos-label { position: absolute; top: 0; left: 0; max-width: 100%; padding: 0 6px; border-radius: 4px; background: var(--accent-main); color: white; font-size: 12px; font-weight: 800; line-height: 24px; }
  .pos-label.bilingual { top: 9.5px; }
  .line-pair + .line-pair { margin-top: 4px; }
  p { margin: 0; overflow-wrap: anywhere; }
  .chinese { font-size: 16px; font-weight: 700; line-height: 24px; }
  .english { font-size: 13px; font-weight: 400; line-height: 18px; color: var(--text-muted); }
  .chinese + .english { margin-top: 1px; }
  .first p { padding-left: var(--meaning-indent); }
  .probe { position: absolute; top: 0; left: 0; width: 100%; padding: 0; visibility: hidden; pointer-events: none; }
  .accessible-text { position: absolute; width: 1px; height: 1px; overflow: hidden; clip-path: inset(50%); white-space: nowrap; }

  @media (max-width: 600px) {
    .pos-label { padding-inline: 5px; font-size: 11px; line-height: 20px; }
    .pos-label.bilingual { top: 7.5px; }
    .line-pair + .line-pair { margin-top: 3px; }
    .chinese { font-size: 13px; line-height: 20px; }
    .english { font-size: 11px; line-height: 14px; }
  }
</style>
