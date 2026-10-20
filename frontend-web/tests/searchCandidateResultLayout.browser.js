// Real initialized word and real candidates; viewport keyboard geometry is simulated.
async (page) => {
  const check=(value,message)=>{if(!value) throw new Error(message);};
  await page.unroute('**/api/v1/suggestions?*');
  await page.unroute('**/api/v1/analyze**');
  await page.goto(`https://de.alan-studio.uk/?result-layout=${Date.now()}`);
  await page.setViewportSize({width:390,height:844});
  const input=page.getByRole('textbox',{name:'查询德语单词或中文含义',exact:true});
  await input.fill('Quittung');
  await page.locator('.suggestion-item').filter({has:page.locator('.q',{hasText:/^Quittung$/})}).first().click();
  await page.locator('.result-identity h1').filter({hasText:'Quittung'}).waitFor();
  await page.getByRole('button',{name:'展开搜索',exact:true}).waitFor();
  const reports=[];
  for (const [width,height] of [[390,844],[320,568],[844,390],[900,650],[1280,900]]) {
    await page.setViewportSize({width,height});
    for(const theme of ['canvas','canvas-dark']) {
      await page.evaluate(theme=>document.documentElement.setAttribute('data-theme',theme),theme);
      if(await page.getByRole('button',{name:'展开搜索',exact:true}).isVisible()) await page.getByRole('button',{name:'展开搜索',exact:true}).click();
      await input.fill('预约');
      await input.focus();
      await page.locator('.suggestion-item .q').first().waitFor();
      await page.waitForFunction(()=>document.querySelector('.suggestion-list')?.getAttribute('aria-busy')==='false' && document.querySelector('.panel-header')?.textContent.includes('中文找词'));
      await page.waitForTimeout(350);
      const measure=()=>page.evaluate(()=>{
        const panel=document.querySelector('.integrated-suggestions').getBoundingClientRect();
        const list=document.querySelector('.suggestion-list');
        return {height:panel.height,bottom:panel.bottom,list:list.clientHeight,doc:document.documentElement.scrollHeight,main:document.querySelector('.main-area').scrollHeight,width:document.documentElement.scrollWidth};
      });
      const before=await measure();
      while(await page.getByRole('button',{name:/加载更多/}).count()) await page.getByRole('button',{name:/加载更多/}).click();
      const after=await measure();
      check(after.bottom<=height+1 && after.width<=width && after.list>0,'Result candidate panel overflow');
      check(Math.abs(before.height-after.height)<2 && before.doc===after.doc && before.main===after.main,'Result page grew with candidates');
      await page.locator('.suggestion-list').evaluate(el=>el.scrollTop=0);
      await page.screenshot({path:`output/playwright/search-result-candidates-${theme}-${width}x${height}.png`});
      reports.push({theme,width,height,...after});
    }
  }
  await page.setViewportSize({width:390,height:844});
  await input.focus();
  await page.evaluate(()=>{
    Object.defineProperty(visualViewport,'height',{configurable:true,value:360});
    Object.defineProperty(visualViewport,'offsetTop',{configurable:true,value:20});
    visualViewport.dispatchEvent(new Event('resize'));
  });
  await page.waitForTimeout(350);
  const keyboard=await page.locator('.integrated-suggestions').boundingBox();
  check(keyboard.y+keyboard.height<=380,'Simulated keyboard hides candidates');
  await page.screenshot({path:'output/playwright/search-result-candidates-simulated-keyboard.png'});
  await page.evaluate(()=>{delete visualViewport.height;delete visualViewport.offsetTop;visualViewport.dispatchEvent(new Event('resize'));});
  return {reports,keyboard,realPhoneKeyboardTested:false};
}
