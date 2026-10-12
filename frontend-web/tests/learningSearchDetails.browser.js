// Run via playwright-cli run-code --filename from the repository root.
async page => {
  const base = 'https://de.alan-studio.uk';
  const entry = await (await page.request.get(base + '/api/v1/entries/2427')).json();
  let fixture = entry;
  const blocked = [];
  const pattern = '**/api/v1/**';
  await page.route(pattern, async route => {
    const request = route.request(), path = new URL(request.url()).pathname;
    if (path.endsWith('/session/v3/start')) {
      await route.fulfill({json: {
        session_id:'browser-detail-check',business_date:'2026-10-12',completed_count:0,total_count:1,is_completed:false,intraday_queue_count:0,next_available_at:null,
        current_word:{entry_id:fixture.entry_id,query_text:fixture.query_text,analysis_markdown:fixture.analysis_markdown,structured_analysis:fixture.structured_analysis,attached_phrase_modules:fixture.attached_phrase_modules,repetitions_left:1,progress:null,phase:'new',appearance_count_today:1},
      }});
    } else if (path.endsWith('/analyze')) {
      await route.fulfill({json:{...fixture,source:'数据库',quality_mode:'default'}});
    } else if (path.endsWith('/analyze/stream')) {
      await route.fulfill({contentType:'text/event-stream',body:'event: complete\ndata: '+JSON.stringify({...fixture,source:'数据库',quality_mode:'default'})+'\n\n'});
    } else if (request.method() === 'GET') await route.continue();
    else { blocked.push(path); await route.abort(); }
  });
  const results = [];
  const wait = () => page.waitForTimeout(350);
  const checkMeanings = async () => page.evaluate(() => {
    const cards = [...document.querySelectorAll('.interleaved-meaning')];
    if (!cards.length) throw Error('Missing bilingual meanings');
    for (const card of cards) for (const language of ['chinese','english']) {
      const expected = card.querySelector('.probe.'+language).textContent;
      const lines = [...card.querySelectorAll('.line-pair .'+language)];
      if (lines.map(e=>e.textContent).join('') !== expected) throw Error('Lost meaning text');
      if (lines.some(e=>e.getBoundingClientRect().height > parseFloat(getComputedStyle(e).lineHeight)+0.5)) throw Error('Double wrapped meaning');
    }
    const scroll = document.querySelector('.scroll-content') || document.querySelector('.main-area');
    if (scroll.scrollWidth > scroll.clientWidth) throw Error('Horizontal overflow '+JSON.stringify({width:innerWidth,scroll:scroll.scrollWidth,client:scroll.clientWidth,bad:[...scroll.querySelectorAll('*')].filter(e=>e.getBoundingClientRect().right>scroll.getBoundingClientRect().right+1).slice(0,10).map(e=>e.className)}));
    return cards.length;
  });
  const openSearch = async () => {
    await page.goto(base + '/');
    await page.getByRole('textbox',{name:'输入德语单词或近似拼写'}).fill('abnehmen');
    await page.getByRole('button',{name:'分析',exact:true}).click();
    await page.locator('.branch-meaning-item, .meaning-row .interleaved-meaning').first().waitFor();
    await page.waitForTimeout(700);
  };
  const screenshot = async (kind, width, theme, section, selector) => {
    await page.locator(selector).first().evaluate((element,kind)=>{
      const scroll=document.querySelector(kind==='learning'?'.scroll-content':'.main-area');
      const offset=kind==='learning'?8:120;
      scroll.scrollTop+=element.getBoundingClientRect().top-scroll.getBoundingClientRect().top-offset;
    },kind);
    await wait();
    await page.screenshot({path:`output/playwright/rich-responsive-references/detail-adaptation-after-${kind}-${width}-${theme}-${section}.png`});
  };
  try {
    await page.goto(base + '/learn');
    const recall = await page.locator('.recall-content h2').evaluate(e=>parseFloat(getComputedStyle(e).fontSize));
    if (recall < 48) throw Error('Recall word compressed');
    await page.getByRole('button',{name:'查看 abnehmen 的答案'}).click();
    await wait();
    const jump = await page.locator('.usage-section').evaluate(e=>Math.abs(e.getBoundingClientRect().top-document.querySelector('.scroll-content').getBoundingClientRect().top));
    if (jump > 2) throw Error('Answer did not jump to usage');
    for (const theme of ['canvas','canvas-dark']) for (const width of [320,390,600,601,1280]) {
      await page.setViewportSize({width,height:844});
      await page.evaluate(t=>document.documentElement.setAttribute('data-theme',t),theme);
      await page.waitForTimeout(700);
      const meanings = await checkMeanings();
      const before = await page.locator('.answer-title').boundingBox();
      for (const top of [0,100000,0]) {
        await page.locator('.scroll-content').evaluate((e,t)=>e.scrollTop=t,top);
        await wait();
        const after=await page.locator('.answer-title').boundingBox();
        if (after.y!==before.y || after.height!==before.height) throw Error('Learning word moved while scrolling');
      }
      const density=await page.locator('.usage-item').first().evaluate(e=>({font:getComputedStyle(e).fontSize,padding:getComputedStyle(e).padding}));
      if (width<=600 && (density.font!=='13px'||density.padding!=='6px')) throw Error('Learning density not adopted');
      const gap=await page.evaluate(()=>document.querySelector('.rating-bar').getBoundingClientRect().top-document.querySelector('.scroll-content').getBoundingClientRect().bottom);
      if(gap<0)throw Error('Rating bar covers content');
      if([390,1280].includes(width)) for(const [section,selector] of [['meanings','.meaning-section'],['usage','.usage-section'],['insights','.insight-section'],['network','.network-section']]) await screenshot('learning',width,theme,section,selector);
      results.push({kind:'learning',width,theme,meanings,density,gap});
    }
    await openSearch();
    for (const theme of ['canvas','canvas-dark']) for (const width of [320,390,600,601,1280]) {
      await page.setViewportSize({width,height:844});
      await page.evaluate(t=>document.documentElement.setAttribute('data-theme',t),theme);
      await page.waitForTimeout(700);
      const meanings = await checkMeanings();
      const alignment = await page.locator('.card-title-row').evaluate(e=>{
        const title=e.querySelector('.card-title').getBoundingClientRect(),plus=e.querySelector('.phrase-add-toggle').getBoundingClientRect();
        return {sameRow:Math.abs(title.y+title.height/2-plus.y-plus.height/2)<2,right:plus.x>title.right};
      });
      if(!alignment.sameRow||!alignment.right)throw Error('Phrase plus is not right of title '+JSON.stringify({width,theme,...alignment}));
      await page.getByRole('button',{name:'添加短语卡片',exact:true}).last().click();
      await wait();
      const inputFits=await page.locator('.phrase-module-control').evaluate(e=>{
        const input=e.querySelector('input').getBoundingClientRect(),r=e.getBoundingClientRect();
        const page=document.querySelector('.main-area');
        return input.width>100&&input.right<=r.right&&page.scrollWidth<=page.clientWidth;
      });
      if(!inputFits)throw Error('Expanded phrase input does not fit');
      if(width===390)await screenshot('search',width,theme,'phrase-expanded','.card-title-row');
      await page.getByRole('button',{name:'关闭短语添加'}).click();
      await wait();
      const density=await page.locator('.usage-item').first().evaluate(e=>({font:getComputedStyle(e).fontSize,padding:getComputedStyle(e).padding}));
      if(width<=600&&(density.font!=='13px'||density.padding!=='6px'))throw Error('Search density not adopted');
      const pills=await page.locator('.inline-actions button').evaluateAll(elements=>elements.map(e=>{
        const s=getComputedStyle(e);return {nowrap:s.whiteSpace==='nowrap',radius:s.borderRadius,width:e.getBoundingClientRect().width};
      }));
      if(pills.some(p=>!p.nowrap)&&width<=600)throw Error('Follow-up label wraps');
      if(await page.locator('.network-label').count()!==3)throw Error('Network categories missing');
      const tag=page.locator('.mini-tag-btn summary').first();
      await tag.focus();await page.keyboard.press('Enter');
      if(await page.locator('.mini-tag-btn').first().getAttribute('open')===null)throw Error('Keyboard cannot expand network');
      await page.keyboard.press('Enter');
      if([390,1280].includes(width))for(const [section,selector] of [['meanings','.card-header'],['usage','.card-title-row'],['insights','.insights-stack'],['network','.word-network'],['followup','.follow-up-wrapper'],['recent','.recent-mini-list']])await screenshot('search',width,theme,section,selector);
      results.push({kind:'search',width,theme,meanings,density,pills});
    }
    await page.setViewportSize({width:390,height:844});
    await page.locator('.main-area').evaluate(e=>e.scrollTop=0);
    await page.locator('.branch-meaning-item').first().click();
    await page.locator('.popover-card').waitFor();
    await openSearch();
    const cdp=await page.context().newCDPSession(page);
    await cdp.send('Emulation.setTouchEmulationEnabled',{enabled:true,maxTouchPoints:2});
    const swipe=async(points)=>{
      await cdp.send('Input.dispatchTouchEvent',{type:'touchStart',touchPoints:[{x:points[0][0],y:points[0][1]}]});
      for(const [x,y] of points.slice(1)){
        await cdp.send('Input.dispatchTouchEvent',{type:'touchMove',touchPoints:[{x,y}]});
        await page.waitForTimeout(40);
      }
      await cdp.send('Input.dispatchTouchEvent',{type:'touchEnd',touchPoints:[]});
      await wait();
    };
    try {
      for(const points of [[[24,400],[60,400],[160,400]],[[100,400],[140,400],[230,400]],[[10,400],[30,450],[110,530]]]){
        await swipe(points);
        if(!await page.locator('.back-btn').count())throw Error('Search closed outside edge gesture');
      }
      await page.locator('.main-area').evaluate(e=>e.scrollTop=0);
      await swipe([[10,650],[10,600],[10,480],[10,320]]);
      if(await page.locator('.main-area').evaluate(e=>e.scrollTop)<50)throw Error('Native search scroll blocked');
      await swipe([[10,400],[40,400],[150,400]]);
      await page.locator('.back-btn').waitFor({state:'detached'});
      await openSearch();
      await swipe([[10,400],[40,400],[150,400]]);
      await page.locator('.back-btn').waitFor({state:'detached'});
    } finally {await cdp.send('Emulation.setTouchEmulationEnabled',{enabled:false});await cdp.detach();}
    fixture=structuredClone(entry);
    fixture.follow_ups=[{id:-1,question:'长追问内容用于检查窄屏边界。'.repeat(5),answer:'说明文字用于检查历史内容。\n\n- 第一条\n- 第二条',created_at:'2026-10-12'}];
    fixture.attached_phrase_modules=[{phrase:'langes Beispiel',host_headword:'abnehmen',source_phrase_entry_id:-1,analysis_markdown:'',attached_at:'2026-10-12',usage_module:{title:'很长的追加短语标题用于检查删除操作边界'.repeat(3),explanation:'追加的说明文字。',example_de:'Ein zusätzliches Beispiel.',example_zh:'追加例句。'}}];
    await openSearch();
    await checkMeanings();
    await page.locator('.usage-item.is-attached').scrollIntoViewIfNeeded();
    const deletion=await page.locator('.attached-trash-btn').evaluate(e=>{
      const r=e.getBoundingClientRect(),s=getComputedStyle(e);
      return s.opacity==='1'&&s.pointerEvents==='auto'&&Boolean(document.elementFromPoint(r.x+r.width/2,r.y+r.height/2)?.closest('.attached-trash-btn'));
    });
    if(!deletion)throw Error('Attached phrase deletion not accessible');
    await page.locator('.follow-up-history').waitFor();
    fixture.structured_analysis.grammar_branches=[];
    fixture.structured_analysis.meanings=[{part_of_speech:'adj.',chinese:'中文独立释义'.repeat(8),english:'An independent meaning. '.repeat(8)},{part_of_speech:'n.',chinese:'只有中文',english:''}];
    await openSearch();
    if(await checkMeanings()!==2)throw Error('Fallback meanings lost');
    if(blocked.length)throw Error('Unexpected business write: '+blocked.join(','));
    return {results,edgeGestures:'passed',fixtures:'attached phrase, history, fallback meanings passed'};
  } finally {
    await page.goto(base+'/library');
    await page.unroute(pattern);
  }
}
