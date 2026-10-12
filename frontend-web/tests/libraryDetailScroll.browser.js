// Run via playwright-cli run-code --filename from the repository root.
async page => {
  await page.goto('https://de.alan-studio.uk/library');
  await page.reload();
  await page.setViewportSize({width:390,height:844});
  await page.getByRole('textbox',{name:'搜索词条'}).fill('abnehmen');
  const open=async()=>{
    await page.locator('article').filter({has:page.getByRole('heading',{name:'abnehmen',exact:true})}).getByRole('button',{name:'详情 '}).click();
    await page.locator('.drawer-panel').waitFor();
    await page.waitForTimeout(400);
  };
  await open();
  const backgroundTop=await page.locator('.main-area').evaluate(e=>e.scrollTop);
  const results=[];
  for(const theme of ['canvas','canvas-dark']){
    await page.evaluate(t=>document.documentElement.setAttribute('data-theme',t),theme);
    for(const width of [320,390]){
      await page.setViewportSize({width,height:844});
      for(const [name,top] of [['top',0],['docked',400],['bottom',100000],['returned',0]]){
        await page.locator('.drawer-panel').evaluate((e,t)=>e.scrollTop=t,top);
        await page.waitForTimeout(250);
        const result=await page.evaluate(()=>{
          const panel=document.querySelector('.drawer-panel'),toolbar=document.querySelector('.detail-toolbar'),close=document.querySelector('.detail-close-floating');
          return {docked:toolbar.classList.contains('docked'),barTop:toolbar.getBoundingClientRect().top,closeTop:close.getBoundingClientRect().top,nav:getComputedStyle(document.querySelector('.bottom-nav')).visibility,backgroundOverflow:getComputedStyle(document.querySelector('.main-area')).overflowY,overflow:panel.scrollWidth>panel.clientWidth};
        });
        if(result.nav!=='hidden'||result.backgroundOverflow!=='hidden'||result.overflow||Math.abs(result.barTop)>1||Math.abs(result.closeTop-10)>1||result.docked!==(top>0))throw Error(JSON.stringify(result));
        if(name!=='returned')await page.screenshot({path:`output/playwright/rich-responsive-references/detail-scroll-${width}-${theme}-${name}.png`});
        results.push({theme,width,name,...result});
      }
    }
  }
  // Large wheel events at both ends must not move the page behind the detail.
  await page.mouse.move(150,400);
  for(const delta of [20000,20000,-20000,-20000]){
    await page.mouse.wheel(0,delta);
    await page.waitForTimeout(100);
    if(await page.locator('.main-area').evaluate(e=>e.scrollTop)!==backgroundTop)throw Error('Background scrolled');
    if(await page.locator('.bottom-nav').evaluate(e=>getComputedStyle(e).visibility)!=='hidden')throw Error('Navigation revealed');
  }
  await page.getByRole('button',{name:'关闭详情'}).click();
  await page.locator('.detail-overlay').waitFor({state:'detached'});
  const restored=await page.evaluate(()=>({nav:getComputedStyle(document.querySelector('.bottom-nav')).visibility,overflow:getComputedStyle(document.querySelector('.main-area')).overflowY}));
  if(restored.nav!=='visible'||restored.overflow!=='auto')throw Error('Background not restored');
  await open();
  if(await page.locator('.detail-toolbar').evaluate(e=>e.classList.contains('docked')))throw Error('Docked state leaked on reopen');
  await page.setViewportSize({width:1280,height:844});
  await page.waitForTimeout(400);
  const closeClickable=await page.locator('.detail-close-floating').evaluate(e=>{
    const r=e.getBoundingClientRect();
    return Boolean(document.elementFromPoint(r.x+r.width/2,r.y+r.height/2)?.closest('.detail-close-floating'));
  });
  if(!closeClickable)throw Error('Desktop close button covered');
  await page.screenshot({path:'output/playwright/rich-responsive-references/detail-scroll-desktop.png'});
  return {results,restored,rapidWheel:'passed',reopen:'passed'};
}
