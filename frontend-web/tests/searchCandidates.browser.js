// Run with playwright-cli run-code --filename=frontend-web/tests/searchCandidates.browser.js.
async (page) => {
  const check = (condition, message) => { if (!condition) throw new Error(message); };
  const reports = [];
  const errors = [];
  page.on('pageerror', error => errors.push(error.message));
  await page.goto(`https://de.alan-studio.uk/?verify=${Date.now()}`);
  const input = page.getByRole('textbox', {name:'查询德语单词或中文含义', exact:true});
  await input.waitFor();
  const calls = [];
  page.on('request', request => { if (request.url().includes('/suggestions?')) calls.push(request.url()); });
  const started = Date.now();
  await input.fill('预约');
  await page.locator('.suggestion-item .q').first().waitFor();
  const elapsed = Date.now() - started;
  check(await page.locator('.suggestion-item .q').count() === 20, 'Initial count !=20');
  await page.screenshot({path:'output/playwright/search-candidates-initial20.png'});
  const geometry = () => page.evaluate(() => {
    const panel=document.querySelector('.integrated-suggestions');
    const list=document.querySelector('.suggestion-list');
    const main=document.querySelector('.main-area');
    const r=panel.getBoundingClientRect();
    return {panel:r.height,bottom:r.bottom,top:r.top,list:list.clientHeight,listScroll:list.scrollHeight,doc:document.documentElement.scrollHeight,main:main.scrollHeight,width:document.documentElement.scrollWidth,viewport:innerWidth};
  });
  for (const theme of ['canvas','canvas-dark']) {
    await page.evaluate(theme => { localStorage.setItem('de-ai-theme',theme); document.documentElement.setAttribute('data-theme',theme); },theme);
    for (const [width,height] of [[1280,900],[900,650],[390,844],[320,568],[844,390]]) {
      await page.setViewportSize({width,height});
      await input.focus();
      await page.waitForTimeout(350);
      const before=await geometry();
      const requests=calls.length;
      while(await page.getByRole('button',{name:/加载更多/}).count()) {
        await page.getByRole('button',{name:/加载更多/}).click();
      }
      const after=await geometry();
      check(await page.locator('.suggestion-item .q').count()===100,'Expanded count !=100');
      check(Math.abs(before.panel-after.panel)<2 && before.doc===after.doc && before.main===after.main,'Candidate growth expanded page');
      check(calls.length===requests,'Load more requested HTTP');
      check(after.bottom<=height+1 && after.width<=width,'Panel overflow');
      check(after.listScroll>after.list && after.list>0,'No internal scrolling');
      await page.locator('.suggestion-list').evaluate(el=>{el.scrollTop=el.scrollHeight;});
      await page.locator('.suggestion-item .q').last().scrollIntoViewIfNeeded();
      await page.screenshot({path:`output/playwright/search-candidates-${theme}-${width}x${height}-end.png`});
      await page.locator('.suggestion-list').evaluate(el=>{el.scrollTop=0;});
      await page.screenshot({path:`output/playwright/search-candidates-${theme}-${width}x${height}.png`});
      reports.push({theme,width,height,...after});
    }
  }
  // A real API result far below the first page must remain reachable.
  check(await page.locator('.suggestion-item .q').filter({hasText:/^Termin$/}).count()===1,'Termin missing from expanded semantic candidates');
  await input.fill('ver');
  await page.waitForFunction(()=>document.querySelector('.panel-header')?.textContent.includes('词汇建议') && document.querySelectorAll('.suggestion-item .q').length===20);
  const lexical=await page.evaluate(()=>fetch('/api/v1/suggestions?q=ver&limit=100').then(r=>r.json()));
  check(lexical.suggestions.length>20,'Lexical upstream remains capped');
  const defaults=await page.evaluate(()=>fetch('/api/v1/suggestions?q=ver').then(r=>r.json()));
  check(defaults.suggestions.length===10,'Default API compatibility');
  for(const query of ['Straße','übernehmen','Ärger','größer','ẞa']) {
    await input.fill(query);
    await page.waitForTimeout(350);
    check(calls.at(-1).includes('mode=lexical'),'German routed semantically');
  }
  await input.fill('');
  const prior=calls.length;
  await input.dispatchEvent('compositionstart');
  await input.fill('ai');
  await input.press('Enter');
  await page.waitForTimeout(300);
  check(calls.length===prior,'IME composition sent requests');
  await input.fill('爱');
  await input.dispatchEvent('compositionend');
  await page.waitForTimeout(1500);
  check(calls.length===prior+1 && calls.at(-1).includes('mode=semantic'),'IME commit should make one semantic request');
  await page.getByRole('button',{name:'关闭搜索建议'}).click({position:{x:1,y:1}});
  await page.waitForTimeout(250);
  check(await page.locator('.integrated-suggestions').count()===0,'Dismissed panel reopened');
  check(errors.length===0,JSON.stringify(errors));
  return {elapsed,lexical:lexical.suggestions.length,reports,requests:calls.length,errors};
}
