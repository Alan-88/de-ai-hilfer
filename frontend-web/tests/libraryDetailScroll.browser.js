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
    for(const width of [320,390,600,601,768,1024,1280]){
      await page.setViewportSize({width,height:844});
      let initial;
      for(const [name,top] of [['top',0],['scrolled',400],['bottom',100000],['returned',0]]){
        await page.locator('.drawer-panel').evaluate((e,t)=>e.scrollTop=t,top);
        await page.waitForTimeout(250);
        const result=await page.evaluate(()=>{
          const panel=document.querySelector('.drawer-panel'),toolbar=document.querySelector('.detail-toolbar'),close=document.querySelector('.detail-close-floating'),word=document.querySelector('.word-title');
          const b=toolbar.getBoundingClientRect(),w=word.getBoundingClientRect(),c=close.getBoundingClientRect();
          return {barTop:b.top,closeTop:c.top,titleTop:w.top,titleSize:parseFloat(getComputedStyle(word).fontSize),titleOffset:w.x+w.width/2-b.x-b.width/2,overlap:c.right>w.left,closeClickable:Boolean(document.elementFromPoint(c.x+c.width/2,c.y+c.height/2)?.closest('.detail-close-floating')),bottomInset:parseFloat(getComputedStyle(panel).borderBottomWidth),nav:getComputedStyle(document.querySelector('.bottom-nav')).visibility,backgroundOverflow:getComputedStyle(document.querySelector('.main-area')).overflowY,overflow:panel.scrollWidth>panel.clientWidth};
        });
        const expectedCloseTop=width<=600?6:14;
        if(result.nav!=='hidden'||result.backgroundOverflow!=='hidden'||result.overflow||Math.abs(result.barTop)>1||Math.abs(result.closeTop-expectedCloseTop)>1||!result.closeClickable||Math.abs(result.titleOffset)>1||result.overlap||result.titleSize!==(width<=600?26:width<=768?32:40)||result.bottomInset!==(width<=600?0:16))throw Error(JSON.stringify(result));
        if(!initial)initial=result;
        if(result.titleTop!==initial.titleTop||result.titleSize!==initial.titleSize)throw Error('Title moved or shrank while scrolling');
        if(name!=='returned')await page.screenshot({path:`output/playwright/rich-responsive-references/fixed-word-header-${width}-${theme}-${name}.png`});
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
  await page.getByRole('button',{name:'返回词库'}).click();
  await page.locator('.detail-overlay').waitFor({state:'detached'});
  const restored=await page.evaluate(()=>({nav:getComputedStyle(document.querySelector('.bottom-nav')).visibility,overflow:getComputedStyle(document.querySelector('.main-area')).overflowY}));
  if(restored.nav!=='visible'||restored.overflow!=='auto')throw Error('Background not restored');
  await open();
  if(await page.locator('.drawer-panel').evaluate(e=>e.scrollTop)!==0)throw Error('Scroll state leaked on reopen');
  await page.setViewportSize({width:1280,height:844});
  await page.waitForTimeout(400);
  const closeClickable=await page.locator('.detail-close-floating').evaluate(e=>{
    const r=e.getBoundingClientRect();
    return Boolean(document.elementFromPoint(r.x+r.width/2,r.y+r.height/2)?.closest('.detail-close-floating'));
  });
  if(!closeClickable)throw Error('Desktop close button covered');
  await page.mouse.click(100,300);
  await page.locator('.detail-overlay').waitFor({state:'detached'});
  await open();
  // Real browser touch input verifies that the narrow edge gesture coexists with native scrolling.
  const cdp=await page.context().newCDPSession(page);
  await cdp.send('Emulation.setTouchEmulationEnabled',{enabled:true,maxTouchPoints:2});
  const swipe=async(points)=>{
    await cdp.send('Input.dispatchTouchEvent',{type:'touchStart',touchPoints:[{x:points[0][0],y:points[0][1]}]});
    for(const [x,y] of points.slice(1)){
      await cdp.send('Input.dispatchTouchEvent',{type:'touchMove',touchPoints:[{x,y}]});
      await page.waitForTimeout(40);
    }
    await cdp.send('Input.dispatchTouchEvent',{type:'touchEnd',touchPoints:[]});
    await page.waitForTimeout(400);
  };
  const staysOpen=async(label)=>{if(!await page.locator('.detail-overlay').count())throw Error('Unexpected close: '+label);};
  try{
    const left=await page.locator('.drawer-shell').evaluate(e=>e.getBoundingClientRect().left);
    await swipe([[left+10,400],[left+40,400],[left+140,400]]);
    await staysOpen('desktop edge');
    await page.setViewportSize({width:390,height:844});
    for(const [label,points] of [
      ['middle',[[100,400],[140,400],[220,400]]],
      ['outside 20px edge',[[24,400],[60,400],[150,400]]],
      ['short edge swipe',[[10,400],[30,400],[50,400]]],
      ['leftward edge swipe',[[19,400],[12,400],[2,400]]],
      ['diagonal scroll',[[10,400],[30,450],[100,520]]],
    ]){await swipe(points);await staysOpen(label);}
    await page.locator('.drawer-panel').evaluate(e=>e.scrollTop=0);
    await swipe([[10,600],[10,560],[10,440],[10,300]]);
    await staysOpen('vertical edge scroll');
    if(await page.locator('.drawer-panel').evaluate(e=>e.scrollTop)<50)throw Error('Native edge scrolling blocked');
    await cdp.send('Input.dispatchTouchEvent',{type:'touchStart',touchPoints:[{x:10,y:400},{x:30,y:450}]});
    await cdp.send('Input.dispatchTouchEvent',{type:'touchMove',touchPoints:[{x:130,y:400},{x:150,y:450}]});
    await cdp.send('Input.dispatchTouchEvent',{type:'touchEnd',touchPoints:[]});
    await staysOpen('multiple touches');
    await cdp.send('Input.dispatchTouchEvent',{type:'touchStart',touchPoints:[{x:10,y:400}]});
    await cdp.send('Input.dispatchTouchEvent',{type:'touchMove',touchPoints:[{x:130,y:400}]});
    await cdp.send('Input.dispatchTouchEvent',{type:'touchCancel',touchPoints:[]});
    await staysOpen('cancelled gesture');
    await swipe([[19,400],[45,401],[90,402],[140,403]]);
    await page.locator('.detail-overlay').waitFor({state:'detached'});
    if(await page.locator('.bottom-nav').evaluate(e=>getComputedStyle(e).visibility)!=='visible')throw Error('Swipe did not restore navigation');
    await open();
    await swipe([[10,30],[40,30],[130,30]]);
    await page.locator('.detail-overlay').waitFor({state:'detached'});
  }finally{
    await cdp.send('Emulation.setTouchEmulationEnabled',{enabled:false});
    await cdp.detach();
  }
  return {results,restored,rapidWheel:'passed',reopen:'passed',outsideClick:'passed',edgeSwipe:'passed'};
}
