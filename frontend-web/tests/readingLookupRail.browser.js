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
    await demo.getByRole('button', {name: '展开搜索'}).click();
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
      return {railWidth: r.width, fontSizes: expected, headTop: bounds.top, articleScroll: document.querySelector('.article-scroll').scrollTop};
    });
    const initial = await inspect();
    if ([390, 1280].includes(width)) await demo.screenshot({path: `output/playwright/reading-rail-search-after-${width}-${theme}.png`});
    await demo.locator('.lookup-scroll').evaluate(item => item.scrollTop = 400);
    const scrolled = await inspect();
    if (scrolled.headTop !== initial.headTop || scrolled.articleScroll !== initial.articleScroll) throw Error('Lookup scrolling moved word or article');
    if ([390, 1280].includes(width)) await demo.screenshot({path: `output/playwright/reading-rail-search-scrolled-${width}-${theme}.png`});
    const headerHeight = await demo.locator('.lookup-head').evaluate(item => item.getBoundingClientRect().height);
    await demo.getByRole('button', {name: '展开搜索'}).click();
    await demo.waitForTimeout(300);
    if (!await input.evaluate(item => item === document.activeElement)) throw Error('Search does not focus input');
    if (await demo.locator('.lookup-head').evaluate(item => item.getBoundingClientRect().height) !== headerHeight) throw Error('Search changes header height');
    if (await demo.locator('.lookup-scroll').evaluate(item => item.scrollTop) !== 400) throw Error('Search opening changes reading position');
    if (await demo.locator('.word-head').getAttribute('aria-hidden') !== 'true') throw Error('Search does not hide word');
    await input.fill('ab');
    await demo.locator('.lookup-suggestions strong').filter({hasText: /^ab$/}).waitFor();
    if ([390, 1280].includes(width)) await demo.screenshot({path: `output/playwright/reading-rail-search-expanded-${width}-${theme}.png`});
    await demo.evaluate(() => {
      Object.defineProperty(visualViewport, 'height', {configurable: true, value: 340});
      visualViewport.dispatchEvent(new Event('resize'));
    });
    const candidates = await demo.locator('.lookup-suggestions').evaluate(item => ({bottom: item.getBoundingClientRect().bottom, scrollable: item.scrollHeight > item.clientHeight}));
    if (candidates.bottom > 340 || !candidates.scrollable) throw Error('Candidates do not fit keyboard viewport');
    await demo.evaluate(() => { delete visualViewport.height; visualViewport.dispatchEvent(new Event('resize')); });
    await input.press('Escape');
    await demo.waitForTimeout(300);
    if (await demo.locator('.lookup-scroll').evaluate(item => item.scrollTop) !== 400) throw Error('Closing search changes reading position');
    if (await demo.locator('.word-head').getAttribute('aria-hidden') !== 'false') throw Error('Word not restored after search');
    for (let i = 0; i < 4; i++) {
      await demo.getByRole('button', {name: '展开搜索'}).click();
      await demo.getByRole('button', {name: '收起搜索'}).click();
      if (await demo.locator('.lookup-head').evaluate(item => item.getBoundingClientRect().height) !== headerHeight) throw Error('Rapid search toggle changes height');
      if (width <= 900 && !await demo.locator('.lookup-rail').evaluate(item => item.classList.contains('mobile-open'))) throw Error('Search back arrow closes mobile rail');
    }
    await demo.emulateMedia({reducedMotion: 'reduce'});
    await demo.getByRole('button', {name: '展开搜索'}).click();
    if (await demo.locator('.lookup-search form').evaluate(item => item.getAnimations().length)) throw Error('Reduced motion still animates search');
    await demo.getByRole('button', {name: '收起搜索'}).click();
    await demo.emulateMedia({reducedMotion: 'no-preference'});
    if ([390, 1280].includes(width) && theme === 'canvas') {
      await demo.getByRole('button', {name: '展开搜索'}).click();
      await demo.locator('.lookup-scroll').click({position: {x: 4, y: 4}});
      if (await demo.locator('.lookup-head').evaluate(item => item.classList.contains('search-open'))) throw Error('Click outside search does not collapse it');
      if (await demo.locator('.lookup-scroll').evaluate(item => item.scrollTop) !== 400) throw Error('Blur changes reading position');

      let release, signalStarted, signalFinished;
      const held = new Promise(resolve => release = resolve);
      const started = new Promise(resolve => signalStarted = resolve);
      const finished = new Promise(resolve => signalFinished = resolve);
      const delayed = async route => {
        const response = await route.fetch();
        signalStarted();
        await held;
        await route.fulfill({response});
        signalFinished();
      };
      await demo.route('**/suggestions?q=ab', delayed);
      await demo.getByRole('button', {name: '展开搜索'}).click();
      await input.fill('ab');
      await started;
      await demo.getByRole('button', {name: '收起搜索'}).click();
      release();
      await finished;
      await demo.waitForTimeout(100);
      if (await demo.locator('.lookup-suggestions').count()) throw Error('Late candidates reopen closed search');
      await demo.unroute('**/suggestions?q=ab', delayed);

      await demo.route('**/suggestions?q=zz-no-candidate', route => route.fulfill({json: {suggestions: []}}));
      await demo.getByRole('button', {name: '展开搜索'}).click();
      await input.fill('zz-no-candidate');
      await input.press('Enter');
      await demo.getByText('未找到可靠候选', {exact: true}).waitFor();
      if (!await demo.locator('.lookup-head').evaluate(item => item.classList.contains('search-open'))) throw Error('Failed lookup closes search');
      await input.fill('abnehmen');
      await demo.locator('.lookup-suggestions button').filter({has: demo.locator('strong').filter({hasText: /^abnehmen$/})}).click();
      await demo.locator('.word-head h2').filter({hasText: /^abnehmen$/}).waitFor();
      await demo.locator('.lookup-head.search-open').waitFor({state: 'hidden'});

      if (width === 1280) {
        const marked = demo.locator('.article-copy .marked:not(.unavailable)').first();
        const expectedWord = await marked.getAttribute('title');
        await demo.getByRole('button', {name: '展开搜索'}).click();
        await marked.click();
        await demo.locator('.lookup-head.search-open').waitFor({state: 'hidden'});
        await demo.locator('.word-head h2').filter({hasText: expectedWord}).waitFor();
      }
    }
    await demo.locator('.lookup-scroll').evaluate(item => item.scrollTop = 0);
    const grammar = demo.locator('.lookup-rail .meaning-summary.interactive').first();
    if (await grammar.count()) {
      await grammar.click();
      await demo.locator('.popover-card').waitFor();
      await demo.keyboard.press('Escape');
      await demo.locator('.popover-card').waitFor({state: 'hidden'});
    }
    if ([320, 1280].includes(width) && theme === 'canvas') {
      const title = demo.locator('.word-head h2');
      const original = await title.textContent();
      await title.evaluate(item => item.textContent = 'Donaudampfschifffahrtsgesellschaftskapitän');
      const longHead = await demo.locator('.lookup-head').evaluate(item => item.getBoundingClientRect().height);
      if (await demo.locator('.lookup-rail').evaluate(item => item.scrollWidth > item.clientWidth)) throw Error('Long word overflows rail');
      await demo.getByRole('button', {name: '展开搜索'}).click();
      if (await demo.locator('.lookup-head').evaluate(item => item.getBoundingClientRect().height) !== longHead) throw Error('Search changes long-word header height');
      await demo.getByRole('button', {name: '收起搜索'}).click();
      await title.evaluate((item, text) => item.textContent = text, original);
    }
    if (width <= 900) {
      await demo.getByRole('button', {name: '关闭查词栏', exact: true}).filter({has: demo.locator('i')}).click();
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
