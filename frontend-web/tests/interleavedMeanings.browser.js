// Run with playwright-cli run-code --filename from the repository root on a deployed library page.
async page => {
  const openWord = async () => {
    await page.goto('https://de.alan-studio.uk/library');
    await page.getByRole('textbox', {name:'搜索词条'}).fill('abnehmen');
    await page.locator('article').filter({has:page.getByRole('heading',{name:'abnehmen',exact:true})}).getByRole('button',{name:'详情 '}).click();
    await page.locator('.meaning-summary').first().waitFor();
    await page.waitForTimeout(700);
  };
  const inspect = async () => page.evaluate(() => {
    const cards=[...document.querySelectorAll('.interleaved-meaning')];
    for(const card of cards){
      for(const language of ['chinese','english']){
        const expected=card.querySelector('.probe.'+language).textContent;
        const lines=[...card.querySelectorAll('.line-pair .'+language)];
        if(lines.map(e=>e.textContent).join('')!==expected)throw Error('Lost '+language+' text');
        if(lines.some(e=>e.getBoundingClientRect().height>parseFloat(getComputedStyle(e).lineHeight)+0.5))throw Error('Unexpected second wrap');
        const expectedSize=innerWidth<=600?(language==='chinese'?13:11):(language==='chinese'?16:13);
        if(parseFloat(getComputedStyle(card.querySelector('.probe.'+language)).fontSize)!==expectedSize)throw Error('Wrong language size');
      }
      const first=card.querySelector('.line-pair');
      const label=card.querySelector('.pos-label');
      if(first.querySelectorAll('p').length===2){
        const a=first.getBoundingClientRect(),b=label.getBoundingClientRect();
        if(Math.abs((a.top+a.bottom)/2-(b.top+b.bottom)/2)>1)throw Error('Label not centered');
      }
    }
    const panel=document.querySelector('.drawer-panel');
    if(panel.scrollWidth>panel.clientWidth)throw Error('Horizontal overflow');
    return {cards:cards.length,lines:cards.map(c=>({zh:c.querySelectorAll('.line-pair .chinese').length,en:c.querySelectorAll('.line-pair .english').length})),fullExamples:[...document.querySelectorAll('.example-box')].filter(e=>{const r=e.getBoundingClientRect();return r.top>=0&&r.bottom<=innerHeight;}).length};
  });
  const results=[];
  await openWord();
  for(const theme of ['canvas','canvas-dark']){
    await page.evaluate(t=>document.documentElement.setAttribute('data-theme',t),theme);
    for(const width of [390,320,600,601,1280,390]){
      await page.setViewportSize({width,height:844});
      await page.waitForTimeout(400);
      const metrics=await inspect();
      if(!metrics.cards)throw Error('Interleaved meanings missing');
      if([320,390,1280].includes(width))await page.screenshot({path:`output/playwright/rich-responsive-references/desktop-library-after-${width}-${theme}-top.png`});
      results.push({theme,width,...metrics});
    }
  }
  await page.locator('.meaning-summary').first().click();
  await page.locator('.popover-card').waitFor();
  await openWord();
  // Browser-only fixtures exercise unequal line counts and missing English without writing data.
  const detailPath='**/api/v1/entries/2427';
  for(const fixture of [
    {zh:'减少',en:'to decrease gradually over a long period; to take responsibility for another person; to remove something from its original position'},
    {zh:'这是用于验证只有中文而没有英文时仍然保持完整显示并且能够自动换行的释义',en:''},
    {zh:'这是用于验证较长词性标签与多行中文释义的文本。'.repeat(3),en:'to decrease gradually over a long period; to remove something from its original position; '.repeat(4),longTitle:'Donaudampfschifffahrtsgesellschaftskapitän'.repeat(2)}
  ]){
    await page.route(detailPath,async route=>{
      const response=await route.fetch();
      const data=await response.json();
      data.structured_analysis.grammar_branches[0].meanings=[fixture];
      if(fixture.longTitle){
        data.query_text=fixture.longTitle;
        Object.assign(data.structured_analysis.grammar_branches[0].grammar,{transitivity:'both',reflexive:'required',separable:'separable'});
      }
      await route.fulfill({response,json:data});
    });
    try{
      for(const width of [320,1280]){
        await page.setViewportSize({width,height:844});
        await openWord();
        const metrics=await inspect();
        const actual=await page.locator('.interleaved-meaning .probe.english').first().textContent();
        if(actual!==fixture.en)throw Error('Fixture was not applied');
        if(fixture.en && !fixture.longTitle && metrics.lines[0].en<=metrics.lines[0].zh)throw Error('Unequal-line fixture did not exercise tail');
        if(fixture.longTitle){
          await page.locator('.drawer-panel').evaluate(e=>e.scrollTop=100000);
          await page.waitForTimeout(200);
          const fits=await page.evaluate(()=>{const w=document.querySelector('.docked-word').getBoundingClientRect(),b=document.querySelector('.detail-toolbar').getBoundingClientRect(),c=document.querySelector('.detail-close-floating').getBoundingClientRect();return Math.abs(w.x+w.width/2-b.x-b.width/2)<1&&w.right<c.left&&Boolean(document.elementFromPoint(c.x+c.width/2,c.y+c.height/2)?.closest('.detail-close-floating'));});
          if(!fits)throw Error('Long docked title overlaps close button');
        }
        results.push({width,fixture:fixture.longTitle?'Long title and label':fixture.en?'English tail':'Chinese only',...metrics});
      }
    }finally{await page.unroute(detailPath);}
  }
  await openWord();
  return results;
}
