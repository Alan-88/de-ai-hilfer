// Run via playwright-cli run-code from the repository root against the deployed frontend.
async page => {
  const origin = 'https://de.alan-studio.uk';
  const index = await (await page.request.get(origin + '/api/v1/reading')).json();
  const article = index.articles.find(item => item.bookmark_count > 0);
  if (!article) throw Error('No existing article available for read-only browser check');
  const results = [];
  for (const width of [320, 390, 900, 1280]) for (const theme of ['canvas', 'canvas-dark']) {
    const context = await page.context().browser().newContext({viewport: {width, height: 900}});
    await context.addInitScript(t => localStorage.setItem('de-ai-theme', t), theme);
    await context.route('**/api/v1/**', route => route.request().method() === 'GET' ? route.continue() : route.abort());
    const demo = await context.newPage();
    await demo.goto(origin + '/learn/articles/' + article.id);
    await demo.locator('.article-copy').waitFor();
    if (width <= 900) await demo.locator('.mobile-lookup').click();
    const input = demo.getByRole('textbox', {name: '查询单词'});
    await input.fill('abnehmen');
    await demo.locator('.lookup-suggestions strong').filter({hasText: /^abnehmen$/}).waitFor();
    await demo.locator('.lookup-suggestions button').filter({has: demo.locator('strong').filter({hasText: /^abnehmen$/})}).click();
    await demo.locator('.word-head h2').filter({hasText: /^abnehmen$/}).waitFor();
    await demo.waitForTimeout(500);
    const inspect = () => demo.evaluate(() => {
      const rail = document.querySelector('.lookup-rail');
      const scroll = rail.querySelector('.lookup-scroll');
      const head = rail.querySelector('.word-head');
      const bounds = head.getBoundingClientRect();
      const r = rail.getBoundingClientRect();
      const cards = [...rail.querySelectorAll('.interleaved-meaning')];
      if (!cards.length) throw Error('Missing subtitle meanings');
      const expected = r.width <= 340 ? [13, 11] : [14, 11.5];
      for (const card of cards) for (const [i, language] of ['chinese', 'english'].entries()) {
        const probe = card.querySelector('.probe.' + language);
        const lines = [...card.querySelectorAll('.line-pair .' + language)];
        if (lines.map(item => item.textContent).join('') !== probe.textContent) throw Error('Meaning text was lost');
        if (parseFloat(getComputedStyle(probe).fontSize) !== expected[i]) throw Error('Density follows viewport instead of rail width');
        if (lines.some(item => item.getBoundingClientRect().height > parseFloat(getComputedStyle(item).lineHeight) + 0.5)) throw Error('Meaning line wrapped twice');
      }
      if (scroll.scrollWidth > scroll.clientWidth) throw Error('Lookup rail overflows horizontally');
      const example = rail.querySelector('.example-box');
      if (!example || getComputedStyle(example).backgroundColor === 'rgba(0, 0, 0, 0)') throw Error('Example background was lost');
      const button = rail.querySelector('.learning-action');
      if (getComputedStyle(button).whiteSpace !== 'nowrap' || button.scrollWidth > button.clientWidth) throw Error('Learning button label wraps');
      if (head.closest('.lookup-scroll')) throw Error('Word header is inside the scrolling content');
      if (Math.abs((bounds.left + bounds.right - r.left - r.right) / 2) > 1) throw Error('Word header not centered');
      return {railWidth: r.width, fontSizes: expected, headTop: bounds.top, articleScroll: document.querySelector('.article-pane').scrollTop};
    });
    const initial = await inspect();
    if ([390, 1280].includes(width)) await demo.screenshot({path: `output/playwright/reading-rail-after-${width}-${theme}.png`});
    await demo.locator('.lookup-scroll').evaluate(item => item.scrollTop = 400);
    const scrolled = await inspect();
    if (scrolled.headTop !== initial.headTop || scrolled.articleScroll !== initial.articleScroll) throw Error('Lookup scrolling moved word or article');
    if ([390, 1280].includes(width)) await demo.screenshot({path: `output/playwright/reading-rail-scrolled-${width}-${theme}.png`});
    await demo.locator('.lookup-scroll').evaluate(item => item.scrollTop = 0);
    const grammar = demo.locator('.lookup-rail .meaning-summary.interactive').first();
    if (await grammar.count()) {
      await grammar.click();
      await demo.locator('.popover-card').waitFor();
      await demo.keyboard.press('Escape');
      await demo.locator('.popover-card').waitFor({state: 'hidden'});
    }
    if (width <= 900) {
      await demo.locator('.mobile-close').click();
      if (await demo.locator('.lookup-rail').getAttribute('class').then(value => value.includes('mobile-open'))) throw Error('Back arrow does not close rail');
      await demo.locator('.mobile-lookup').click();
      await demo.locator('.rail-dismiss').click({position: {x: 3, y: 200}});
      if (await demo.locator('.lookup-rail').getAttribute('class').then(value => value.includes('mobile-open'))) throw Error('Outside click does not close rail');
    }
    results.push({width, theme, ...initial, scrollFixed: true});
    await context.close();
  }
  return results;
}
