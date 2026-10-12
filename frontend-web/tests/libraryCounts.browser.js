// Run via playwright-cli run-code --filename from the repository root.
async page => {
  await page.goto('https://de.alan-studio.uk/library');
  await page.reload();
  const tabs = ['all', 'learning', 'review', 'new'];
  const results = [];
  for (const q of ['', 'apfel', 'zzmissingcounts']) {
    let counts;
    for (const tab of tabs) {
      const response = await page.request.get('https://de.alan-studio.uk/api/v1/entries', {
        params: {q, tab, limit: 1},
      });
      if (!response.ok()) throw Error(`API ${response.status()}`);
      const body = await response.json();
      const c = body.counts;
      if (!c || tabs.some(t => !Number.isInteger(c[t]) || c[t] < 0)
          || c.all !== c.learning + c.new || c.review > c.learning
          || body.total !== c[tab] || body.items.length > 1) throw Error(JSON.stringify(body));
      if (counts && JSON.stringify(c) !== JSON.stringify(counts)) throw Error('Counts vary by tab');
      counts = c;
    }
    results.push({q, counts});
  }
  const counts = results[0].counts;
  const statsResponse = await page.request.get('https://de.alan-studio.uk/api/v1/learning/stats');
  const stats = await statsResponse.json();
  if (stats.due_today !== counts.review || stats.total_words !== counts.learning) throw Error('Learning stats mismatch');
  await page.waitForFunction(c => [...document.querySelectorAll('.tab-btn .count')].every((e, i) => e.textContent === String(c[['all', 'learning', 'review', 'new'][i]])), counts);
  for (const theme of ['canvas', 'canvas-dark']) {
    await page.evaluate(t => document.documentElement.setAttribute('data-theme', t), theme);
    for (const width of [320, 390, 1280]) {
      await page.setViewportSize({width, height: 844});
      await page.locator('.main-area').evaluate(e => e.scrollTop = 0);
      const fits = await page.locator('.tab-btn').evaluateAll(es => es.every(e => e.scrollWidth <= e.clientWidth && getComputedStyle(e.querySelector('.count')).display !== 'none'));
      if (!fits) throw Error('Tab count hidden or overflowing');
      await page.screenshot({path: `output/playwright/rich-responsive-references/library-four-counts-${width}-${theme}.png`});
      await page.locator('.tabs-row').screenshot({path: `output/playwright/rich-responsive-references/library-four-counts-${width}-${theme}-tabs.png`});
    }
  }
  for (let i = 1; i < tabs.length; i++) {
    const loaded = page.waitForResponse(r => new URL(r.url()).pathname === '/api/v1/entries' && new URL(r.url()).searchParams.get('tab') === tabs[i]);
    await page.locator('.tab-btn').nth(i).click();
    await loaded;
    await page.waitForFunction(total => document.querySelector('.results-meta')?.textContent?.endsWith(`/ ${total}`) || (total === 0 && Boolean(document.querySelector('.empty-state'))), counts[tabs[i]]);
  }
  const loaded = page.waitForResponse(r => new URL(r.url()).pathname === '/api/v1/entries' && new URL(r.url()).searchParams.get('q') === 'zzmissingcounts');
  await page.getByRole('textbox', {name: '搜索词条'}).fill('zzmissingcounts');
  await loaded;
  await page.waitForFunction(() => [...document.querySelectorAll('.tab-btn .count')].every(e => e.textContent === '0') && Boolean(document.querySelector('.empty-state')));
  await page.getByRole('textbox', {name: '搜索词条'}).fill('');
  return {results, stats, themesAndBreakpoints: 'passed', tabSwitching: 'passed', emptySearch: 'passed'};
}
