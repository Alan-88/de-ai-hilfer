// Run via playwright-cli run-code against the deployed frontend; all business writes are blocked.
async page => {
  const phase = 'after';
  const origin = 'https://de.alan-studio.uk';
  const index = await (await page.request.get(origin + '/api/v1/reading')).json();
  const summary = index.articles.find(item => item.bookmark_count > 0);
  if (!summary) throw Error('No bookmarked article available');
  const articleUrl = origin + '/api/v1/reading/articles/' + summary.id;
  const article = await (await page.request.get(articleUrl)).json();
  const results = [];
  for (const [width, height] of [[320, 900], [390, 900], [1280, 900], [390, 540]]) for (const theme of ['canvas', 'canvas-dark']) {
    if (phase === 'before' && (width === 320 || height !== 900)) continue;
    const context = await page.context().browser().newContext({viewport: {width, height}});
    await context.addInitScript(t => localStorage.setItem('de-ai-theme', t), theme);
    await context.route('**/api/v1/**', route => route.request().method() === 'GET' ? route.continue() : route.abort());
    const demo = await context.newPage();
    await demo.goto(origin + '/learn/articles/' + summary.id);
    await demo.locator('.article-copy').waitFor();
    await demo.evaluate(() => document.fonts.ready);
    await demo.waitForTimeout(350);
    const trigger = demo.locator('.bookmark-menu > button');
    if (height === 900 && width !== 320) {
      await demo.screenshot({path: `output/playwright/reading-typography-${phase}-${width}-${theme}.png`});
      await trigger.click();
      await demo.locator('.bookmark-popover').waitFor();
      await demo.screenshot({path: `output/playwright/reading-typography-menu-${phase}-${width}-${theme}.png`});
      await trigger.click();
    }
    if (phase === 'after') {
      const typography = await demo.evaluate(() => {
        const body = getComputedStyle(document.querySelector('.article-copy'));
        const title = getComputedStyle(document.querySelector('.title-line h1'));
        const path = document.querySelector('.folder-path');
        return {bodySize: parseFloat(body.fontSize), bodyLine: parseFloat(body.lineHeight), titleSize: parseFloat(title.fontSize), pathVisible: getComputedStyle(path).display !== 'none', path: path.textContent};
      });
      if (width <= 600 && (typography.bodySize !== 18 || Math.abs(typography.bodyLine - 34.2) > 0.1 || Math.abs(typography.titleSize - 18.4) > 0.1)) throw Error('Mobile reading typography is incorrect');
      if (width > 900 && (!typography.pathVisible || !typography.path.trim() || typography.bodySize < 20)) throw Error('Desktop path or reading typography is incorrect');
      if (width <= 900 && typography.pathVisible) throw Error('Folder path crowds mobile title');
      if (!await trigger.evaluate(item => Boolean(item.closest('.article-foot')))) throw Error('Bookmark trigger is not in the footer');
      if (await demo.locator('.page-head .bookmark-menu, .page-head .mobile-lookup').count()) throw Error('Actions still crowd the title');
      if (await demo.locator('.article-foot').evaluate(item => item.getBoundingClientRect().height) !== 44) throw Error('Footer is not compact');
      const longArticle = {...article, title: article.title + ' — Eine sehr lange Überschrift für den schmalen Bildschirm', segments: [...article.segments, {kind: 'text', text: ('\n\n' + article.segments.map(item => item.kind === 'text' ? item.text : item.surface).join('')).repeat(12)}]};
      await demo.route(articleUrl, route => route.fulfill({json: longArticle}));
      await demo.reload();
      await demo.locator('.article-copy').waitFor();
      await demo.evaluate(() => document.fonts.ready);
      if (width <= 600 && await demo.locator('.title-line h1').evaluate(item => item.getBoundingClientRect().height > parseFloat(getComputedStyle(item).lineHeight) * 2 + 1)) throw Error('Mobile title exceeds two lines');
      const measure = () => demo.evaluate(() => {
        const pane = document.querySelector('.article-pane');
        const paneTop = pane.getBoundingClientRect().top;
        const scroll = pane.querySelector('.article-scroll');
        const head = pane.querySelector('.page-head').getBoundingClientRect();
        const foot = pane.querySelector('.article-foot').getBoundingClientRect();
        const body = scroll.getBoundingClientRect();
        if (pane.scrollWidth > pane.clientWidth || scroll.scrollWidth > scroll.clientWidth) throw Error('Article overflows horizontally');
        if (body.top < head.bottom - 1 || body.bottom > foot.top + 1) throw Error('Body overlaps fixed bars');
        return {headTop: head.top - paneTop, footTop: foot.top - paneTop, scrollTop: scroll.scrollTop, scrollable: scroll.scrollHeight > scroll.clientHeight};
      });
      const initial = await measure();
      if (!initial.scrollable) throw Error('Long article cannot scroll');
      await demo.locator('.article-scroll').evaluate(item => item.scrollTop = item.scrollHeight);
      const scrolled = await measure();
      if (scrolled.scrollTop <= 0 || Math.abs(scrolled.headTop - initial.headTop) > 0.5 || Math.abs(scrolled.footTop - initial.footTop) > 0.5) throw Error('Scrolling moves fixed bars: ' + JSON.stringify({initial, scrolled}));
      await trigger.click();
      const popover = await demo.locator('.bookmark-popover').evaluate(item => {
        const panel = item.getBoundingClientRect();
        const pane = item.closest('.article-pane');
        const head = pane.querySelector('.page-head').getBoundingClientRect();
        const foot = pane.querySelector('.article-foot').getBoundingClientRect();
        if (panel.top < head.bottom - 1 || panel.bottom > foot.top + 1) throw Error('Bookmarks do not fit between fixed bars');
        if (item.scrollHeight <= item.clientHeight) throw Error('Bookmark list cannot scroll');
        item.scrollTop = item.scrollHeight;
        return {height: panel.height, scrolled: item.scrollTop > 0};
      });
      if (!popover.scrolled) throw Error('Bookmark scroll is ineffective');
      const jump = demo.locator('.bookmark-jump').last();
      const target = await jump.locator('strong').textContent();
      await jump.click();
      await demo.locator('.bookmark-popover').waitFor({state: 'hidden'});
      await demo.locator('.word-head h2').filter({hasText: target}).waitFor();
      if (width <= 900) {
        await demo.locator('.lookup-rail .rail-back').click();
        await demo.getByRole('button', {name: '快捷查词', exact: true}).click();
        if (!await demo.locator('.lookup-rail').evaluate(item => item.classList.contains('mobile-open'))) throw Error('Footer lookup button cannot open rail');
        await demo.locator('.lookup-rail .rail-back').click();
      }
      await trigger.click();
      await demo.locator('.article-foot > span').click();
      if (await demo.locator('.bookmark-popover').count()) throw Error('Outside click does not close bookmarks');
      results.push({width, height, theme, ...typography, barsFixed: true, bookmarkScroll: true});
      if (width === 320 && theme === 'canvas') {
        await demo.unroute(articleUrl);
        await demo.route(articleUrl, route => route.fulfill({json: {...article, segments: [{kind: 'text', text: 'Ein kurzer Text.'}], bookmark_count: 0}}));
        await demo.reload();
        await demo.locator('.article-copy').waitFor();
        if (!await trigger.isDisabled() || !await trigger.textContent().then(value => value.includes('0 标记'))) throw Error('Empty bookmark control is incorrect');
      }
    }
    await context.close();
  }
  return results;
}
