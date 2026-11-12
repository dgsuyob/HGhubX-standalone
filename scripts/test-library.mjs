// Real React app and real synthetic MP4; only the native IPC boundary is replaced.
import assert from 'node:assert/strict';
import { readFile, mkdir, writeFile } from 'node:fs/promises';
import { resolve, join } from 'node:path';
import { createServer } from 'vite';
import { chromium } from 'playwright-core';

const root=resolve(import.meta.dirname,'..');
const output=join(root,'test-results','library',new Date().toISOString().replace(/[:.]/g,'-'));
await mkdir(output,{recursive:true});
const clip=await readFile(join(root,'src-tauri/resources/demo.mp4'));
const vite=await createServer({root,server:{host:'127.0.0.1',port:0,strictPort:false}});
await vite.listen();
const origin=vite.resolvedUrls.local[0];
const browser=await chromium.launch({headless:true,executablePath:process.env.BROWSER_EXECUTABLE||'/usr/bin/google-chrome',args:['--autoplay-policy=no-user-gesture-required']});
const report={transport:'real React + synthetic MP4; native IPC fixture',passed:[],pageErrors:[],external:[]};
try {
 for(const mode of ['desktop','mobile']) {
  const context=await browser.newContext(mode==='mobile'?{viewport:{width:393,height:852},isMobile:true,hasTouch:true,userAgent:'Mozilla/5.0 (iPhone; CPU iPhone OS 18_0 like Mac OS X) AppleWebKit/605.1.15 Version/18.0 Mobile Safari/604.1'}:{viewport:{width:1280,height:820},userAgent:'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 Chrome/140.0.0.0 Safari/537.36'});
  await context.addInitScript(({videoBase64})=>{
   const videoURL=URL.createObjectURL(new Blob([Uint8Array.from(atob(videoBase64),c=>c.charCodeAt(0))],{type:"video/mp4"}));
   const caps={catalog:true,search:true,categories:true,related:true,account:false,comments:false,posts:false,remote_interactions:false,share:false};
   const empty={id:'',name:'',base:'',adapter:'',activated:false,capabilities:{},allow_private_network:false};
   let library=JSON.parse(sessionStorage.getItem('fixture-library')||'null');
   const fixture={calls:[],unexpected:[]};
   const gate=()=>library?{...empty,id:'import:'+library.id,name:library.name,adapter:'playlist_v1',activated:true,capabilities:caps}:empty;
   const card=i=>({...i,cover:'',score:0,hot:0,like_count:0,collect_count:0,comment_count:0,episode_count:i.episodes.length,total_episodes:i.episodes.length,is_finished:true,is_original:0});
   const paged=items=>({items:items.map(card),pagination:{page:1,pages:1,size:30,total:items.length}});
   const save=value=>{library=value;sessionStorage.setItem('fixture-library',JSON.stringify(value));return gate();};
   const invoke=async(cmd,args={})=>{
    fixture.calls.push({cmd,command:args.command?.cmd});
    if(cmd==='gate_state')return gate();
    if(cmd==='source_import_manifest'){
     let v;try{v=JSON.parse(args.text);}catch{throw '请选择有效的媒体清单 JSON 文件';}
     if(v.schema_version!==1||!v.items?.length)throw '清单格式不正确';
     return save(v);
    }
    if(cmd==='source_open_files')return null;
    if(cmd==='source_open_demo')return save({id:'demo',name:'本地演示',items:[{id:1,title:'本地演示视频',tags:['示例'],episodes:[{url:'local'}]}]});
    if(cmd==='source_reload')return gate();
    if(cmd==='gate_reset'){library=null;sessionStorage.removeItem('fixture-library');return empty;}
    if(cmd==='gate_open')throw '响应不是有效的媒体清单 JSON';
    if(cmd==='data_call'){
     const c=args.command;
     if(c.cmd==='boot')return {gate:gate(),base:'',user:null,logged_in:false,strict_security:true,cli_relaxed:false};
     if(!library)throw '请先选择媒体库';
     if(c.cmd==='feed')return paged(library.items);
     if(c.cmd==='tags')return {categories:[{name:'媒体标签',tags:[...new Set(library.items.flatMap(i=>i.tags||[]))].map((name,i)=>({id:i+1,name,slug:name}))}]};
     if(c.cmd==='category'||c.cmd==='tag_videos')return paged(library.items.filter(i=>(i.tags||[]).includes(c.cate||c.tag)));
     if(c.cmd==='search')return {...paged(library.items.filter(i=>i.title.includes(c.q)||(i.tags||[]).some(t=>t.includes(c.q)))),keyword:c.q};
     const item=library.items.find(i=>i.id===c.id);
     if(c.cmd==='video'){if(!item)throw '没有这个条目';return {...card(item),description:'合成测试画面',episodes:item.episodes.map((e,i)=>({ep_num:i+1,title:e.title||'播放',duration:8,status:1})),counts:{collect:0,comment:0,hot:0,like:0},author:{id:0,name:'本机素材'}};}
     if(c.cmd==='play')return {id:c.id,ep_num:c.ep,title:item?.title,duration:8,video_url:videoURL,streams:{main:videoURL}};
     if(c.cmd==='related')return {items:library.items.filter(i=>i.id!==c.id).map(card)};
     if(c.cmd==='logout')return {ok:true};
     fixture.unexpected.push(c.cmd);throw '当前媒体库不提供此能力';
    }
    if(cmd==='playback_log_status')return {enabled:false,path:null,error:null};
    if(cmd.startsWith('playback_log_')||cmd==='debug_log'||cmd==='set_text_interaction'||cmd==='set_orientation'||cmd==='cache_clear')return;
    if(cmd==='plugin:event|listen')return 1;
    if(cmd.startsWith('plugin:'))return false;
    fixture.unexpected.push(cmd);throw 'Unexpected IPC: '+cmd;
   };
   window.__libraryFixture=fixture;
   window.__TAURI_EVENT_PLUGIN_INTERNALS__={unregisterListener(){}};
   window.__TAURI_INTERNALS__={invoke,metadata:{currentWindow:{label:'main'},currentWebview:{label:'main'}},transformCallback:()=>1,unregisterCallback:()=>{}};
  },{videoBase64:clip.toString("base64")});
  const page=await context.newPage();
  page.on('pageerror',e=>report.pageErrors.push(`${mode}: ${e.message}`));
  await page.route('**/*',async route=>{
   const url=route.request().url();
   if(url.startsWith('http://hg.localhost/')||url.startsWith('http://127.0.0.1:17650/')){
    const range=route.request().headers().range;
    const match=range?.match(/^bytes=(\d+)-(\d*)$/);
    if(match){const start=Number(match[1]),end=Math.min(match[2]?Number(match[2]):clip.length-1,clip.length-1);await route.fulfill({status:206,headers:{'content-type':'video/mp4','accept-ranges':'bytes','content-range':`bytes ${start}-${end}/${clip.length}`,'access-control-allow-origin':'*'},body:clip.subarray(start,end+1)});}
    else await route.fulfill({status:200,headers:{'content-type':'video/mp4','accept-ranges':'bytes','access-control-allow-origin':'*'},body:clip});
    return;
   }
   if(new URL(url).origin===new URL(origin).origin)return route.continue();
   report.external.push(url);return route.abort();
  });
  await page.goto(origin);
  await page.getByRole('heading',{name:'从你的媒体库开始'}).waitFor();
  assert.equal(await page.evaluate(()=>window.__libraryFixture.calls.filter(c=>c.cmd==='data_call'&&c.command!=='boot').length),0,'Unconfigured app must not fetch content');
  await page.locator('.a-rise').evaluateAll(async nodes=>{await Promise.all(nodes.flatMap(n=>n.getAnimations()).map(a=>a.finished));});
  await page.screenshot({path:join(output,`${mode}-gate.png`),fullPage:true});
  await page.getByLabel('选择媒体清单文件').setInputFiles({name:'broken.json',mimeType:'application/json',buffer:Buffer.from('{')});
  await page.getByRole('alert').filter({hasText:'有效的媒体清单'}).waitFor();
  const library=(id,name,title)=>({schema_version:1,id,name,items:[{id:1,title,tags:['我的视频'],episodes:[{url:'https://media.example.com/owned.mp4',title:'第一集'},{url:'https://media.example.com/owned-2.mp4',title:'第二集'}]}]});
  const importList=async value=>page.getByLabel('选择媒体清单文件').setInputFiles({name:value.id+'.json',mimeType:'application/json',buffer:Buffer.from(JSON.stringify(value))});
  const a=library('library-a','媒体库 A','我的视频 A'),b=library('library-b','媒体库 B','我的视频 B');
  await importList(a);
  await page.getByRole('navigation',{name:'主导航'}).waitFor();
  await page.waitForFunction(()=>[...document.querySelectorAll('video')].some(v=>v.readyState>=2&&v.duration>0));
  await page.waitForFunction(()=>[...document.querySelectorAll('video')].some(v=>v.currentTime>.1));
  assert.equal(await page.getByRole('button',{name:'评论',exact:true}).count(),0,'Unsupported comments must not render');
  assert.equal(await page.getByRole('button',{name:'排行榜',exact:true}).count(),0,'Unsupported ranks must not render');
  await page.getByRole('button',{name:'收藏',exact:true}).first().click();
  await page.waitForFunction(()=>JSON.parse(localStorage.getItem('app_data_v1:import:library-a:library')||'{}').favs?.length===1);
  assert.equal(await page.getByText('未登录',{exact:true}).count(),0,'Local favorite must not prompt login');
  await page.screenshot({path:join(output,`${mode}-playback.png`),fullPage:true});
  const nav=page.getByRole('navigation',{name:'主导航'});
  await nav.getByRole('button',{name:'搜索',exact:true}).click();
  await page.getByLabel('搜索媒体库').fill('我的视频 A');
  await page.getByLabel('搜索媒体库').press('Enter');
  await page.getByText('我的视频 A',{exact:true}).filter({visible:true}).first().waitFor();
  await nav.getByRole('button',{name:'分类',exact:true}).click();
  await page.getByRole('button',{name:/我的视频.*浏览媒体/}).waitFor();
  const openPicker=async()=>{
   if(mode==='mobile'){await nav.getByRole('button',{name:'我的',exact:true}).click();await page.getByRole('button',{name:'设置',exact:true}).click();}
   else await nav.getByRole('button',{name:'设置',exact:true}).click();
   await page.getByRole('button',{name:'更换媒体库',exact:true}).click();
  };
  await openPicker();await importList(b);
  await page.waitForFunction(()=>[...document.querySelectorAll('video')].some(v=>v.readyState>=2));
  await nav.getByRole('button',{name:'我的',exact:true}).click();
  await page.getByText('媒体库 B',{exact:true}).waitFor();
  assert.equal(await page.evaluate(()=>JSON.parse(localStorage.getItem('app_data_v1:import:library-b:library')||'{}').favs?.length),0,'Same ID in B must not inherit A favorite');
  await openPicker();await importList(a);
  await nav.getByRole('button',{name:'我的',exact:true}).click();
  await page.getByText(/1 部收藏/).first().waitFor();
  await page.reload();await page.getByRole('navigation',{name:'主导航'}).waitFor();
  await page.getByRole('navigation',{name:'主导航'}).getByRole('button',{name:'我的',exact:true}).click();
  await page.getByText(/1 部收藏/).first().waitFor();
  await page.screenshot({path:join(output,`${mode}-library.png`),fullPage:true});
  assert.deepEqual(await page.evaluate(()=>window.__libraryFixture.unexpected),[]);
  report.passed.push(`${mode}: no-source startup, invalid import, MP4 playback, local favorite, search, tags, A/B same-ID isolation, reconnect and restart`);
  await context.close();
 }
 const statePage=await browser.newPage();
 await statePage.goto(`${origin}tests/library-state.html`);
 await statePage.waitForFunction(()=>window.libraryStateFixture?.select);
 await statePage.evaluate(()=>window.libraryStateFixture.select("scope-a"));
 await statePage.waitForFunction(()=>window.libraryStateFixture.state.id==="scope-a");
 await statePage.evaluate(()=>{window.libraryStateFixture.favorite(1);window.libraryStateFixture.capture();});
 await statePage.waitForFunction(()=>window.libraryStateFixture.state.favs.length===1);
 await statePage.evaluate(()=>window.libraryStateFixture.select("scope-b"));
 await statePage.waitForFunction(()=>window.libraryStateFixture.state.id==="scope-b");
 await statePage.evaluate(()=>window.libraryStateFixture.late());
 await statePage.evaluate(()=>new Promise(resolve=>requestAnimationFrame(()=>requestAnimationFrame(resolve))));
 assert.deepEqual(await statePage.evaluate(()=>window.libraryStateFixture.state.history),[],"Late callback from A must not write history into B");
 await statePage.evaluate(()=>window.libraryStateFixture.select("scope-a"));
 await statePage.waitForFunction(()=>window.libraryStateFixture.state.id==="scope-a");
 assert.deepEqual(await statePage.evaluate(()=>window.libraryStateFixture.state.favs),[1]);
 await statePage.close();
 report.passed.push("Real React store: late callbacks retain their source scope; same-ID libraries and reconnects remain isolated");
 assert.deepEqual(report.external,[],'No upstream content services may be contacted');
 assert.deepEqual(report.pageErrors,[]);
 console.log(JSON.stringify(report,null,2));
}finally{await writeFile(join(output,'report.json'),JSON.stringify(report,null,2));await browser.close();await vite.close();}
