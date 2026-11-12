import { useState, type ReactNode } from "react";
import { api, APP_VERSION, hostOf } from "../api";
import { APP_NAME } from "../branding";
import { useStore } from "../store";
import { Modal, Toggle } from "../components/ui";
import GateDialog from "../components/GateDialog";
import { copyPlaybackLog, initPlaybackLog, setPlaybackLog, usePlaybackLog } from "../player/diagnostics";
import { formatNetDiag } from "../netDiagReport";
import { copyText } from "../clipboard";
import { IBack, ISettings, Logo } from "../icons";

function Section({title,children}:{title:string;children:ReactNode}) {
  return <section className="mt-5"><h2 className="text-[11px] text-mut mb-2 px-1">{title}</h2><div className="rounded-(--r-md) bg-s2 divide-y divide-s3">{children}</div></section>;
}
function Row({title,description,children}:{title:string;description?:string;children:ReactNode}) {
  return <div className="flex items-center gap-4 p-4"><div className="flex-1 min-w-0"><div className="text-[13px] text-on">{title}</div>{description&&<div className="text-[11px] text-mut mt-1 leading-relaxed break-words">{description}</div>}</div>{children}</div>;
}
export default function Settings({onBack,wide}:{onBack:()=>void;wide:boolean}) {
  const {theme,setTheme,gate,selectSource,resetGate,favs,history,muteOnStartup,setMuteOnStartup,swipeEp,setSwipeEp,preferHevc,setPreferHevc,toast}=useStore();
  const [picker,setPicker]=useState(false),[disconnect,setDisconnect]=useState(false),[busy,setBusy]=useState(false),[report,setReport]=useState("");
  const logs=usePlaybackLog();
  const run=async(action:()=>Promise<void>)=>{if(busy)return;setBusy(true);try{await action();}catch(e){toast(e instanceof Error?e.message:String(e),"info");}finally{setBusy(false);}};
  const button="text-[12px] text-primary pressable disabled:opacity-50 shrink-0";
  return <div className="absolute inset-0 bg-bg flex flex-col">
    <header className="flex items-center gap-3 px-4 pb-3" style={{paddingTop:"calc(var(--sat, 0px) + 12px)"}}><button aria-label="返回" onClick={onBack} className="w-10 h-10 flex items-center justify-center text-on pressable"><IBack size={22}/></button><ISettings size={23} className="text-primary"/><h1 className="text-[23px] font-bold text-on">设置</h1></header>
    <main className={`flex-1 scroll-y px-4 pb-12 ${wide?"max-w-[760px] w-full mx-auto":""}`}>
      <Section title="媒体库">
        <Row title={gate.name||"未连接"} description={gate.base?hostOf(gate.base):gate.adapter==="demo"?"自带的本机演示视频":"本机文件与导入清单"}><button className={button} onClick={()=>setPicker(true)}>更换媒体库</button></Row>
        {gate.base&&<Row title="刷新媒体清单" description="从你已选择的地址重新读取清单"><button disabled={busy} className={button} onClick={()=>void run(async()=>{selectSource(await api.sourceReload());toast("清单已刷新","ok");})}>刷新</button></Row>}
        <Row title="断开当前媒体库" description="保留该媒体库的收藏与观看记录"><button className={button} onClick={()=>setDisconnect(true)}>断开</button></Row>
      </Section>
      <Section title="外观与播放">
        <Row title="深色外观"><Toggle label="深色外观" on={theme==="dark"} onChange={on=>setTheme(on?"dark":"light")}/></Row>
        <Row title="启动时静音" description="下次打开媒体库时生效"><Toggle label="启动时静音" on={muteOnStartup} onChange={setMuteOnStartup}/></Row>
        <Row title="上下划切集" description="多集媒体上下划换集，划到头再切换条目"><Toggle label="上下划切集" on={swipeEp} onChange={setSwipeEp}/></Row>
        <Row title="优先高清" description="来源提供额外高清流时优先使用"><Toggle label="优先高清" on={preferHevc} onChange={setPreferHevc}/></Row>
      </Section>
      <Section title="本机数据">
        <Row title="当前媒体库记录" description={`${favs.length} 部收藏 · ${history.length} 条观看记录`}><span className="text-[11px] text-mut">独立保存</span></Row>
        <Row title="清理播放缓存" description="保留媒体库、收藏和观看记录"><button disabled={busy} className={button} onClick={()=>void run(async()=>{await api.clearCache();toast("播放缓存已清理","ok");})}>清理</button></Row>
      </Section>
      <Section title="诊断">
        {gate.base&&<Row title="网络诊断" description="结果先在本机展示，由你选择是否复制"><button disabled={busy} className={button} onClick={()=>void run(async()=>{setReport(formatNetDiag(await api.netDiag()));})}>检查</button></Row>}
        <Row title="播放诊断日志" description="仅在本机记录播放事件，默认关闭"><Toggle label="播放诊断日志" on={logs.enabled} onChange={on=>void run(async()=>{await initPlaybackLog();await setPlaybackLog(on);})}/></Row>
        <Row title="复制播放诊断"><button disabled={busy} className={button} onClick={()=>void run(async()=>{await copyPlaybackLog();toast("诊断已复制，请检查后再分享","ok");})}>复制</button></Row>
      </Section>
      <Section title="关于">
        <Row title={`${APP_NAME} ${APP_VERSION}`} description="跨平台媒体播放器与个人媒体库客户端"><Logo size={38}/></Row>
        <p className="p-4 text-[11px] text-mut leading-relaxed">媒体由你选择。软件会为正常播放产生临时缓存，收藏和观看记录保存在本机。请连接你拥有或已获授权的内容。本预览版通过 GitHub Actions 构建，不启用在线更新。</p>
        <p className="p-4 text-[11px] text-mut">PolyForm Noncommercial 1.0.0 · 代码许可证不授予第三方内容权利</p>
      </Section>
    </main>
    <GateDialog open={picker} onClose={()=>setPicker(false)}/>
    <Modal open={disconnect} onClose={()=>setDisconnect(false)} width={360}><div className="p-5"><h2 className="text-[16px] font-semibold text-on">断开当前媒体库？</h2><p className="text-[12px] text-mut mt-3">收藏与观看记录会保留，重新连接同一媒体库后可继续使用。</p><div className="flex gap-3 mt-5"><button className="flex-1 h-10 bg-s3 rounded-(--r-sm) text-on" onClick={()=>setDisconnect(false)}>取消</button><button disabled={busy} className="flex-1 h-10 bg-primary text-on-primary rounded-(--r-sm)" onClick={()=>void run(async()=>{await resetGate();})}>断开</button></div></div></Modal>
    <Modal open={!!report} onClose={()=>setReport("")} width={620}><div className="p-5"><h2 className="font-semibold text-on">网络诊断</h2><pre className="whitespace-pre-wrap break-words text-[11px] text-mut max-h-[60vh] scroll-y my-4">{report}</pre><div className="flex gap-3"><button className="flex-1 h-10 bg-s3 text-on rounded-(--r-sm)" onClick={()=>setReport("")}>关闭</button><button className="flex-1 h-10 bg-primary text-on-primary rounded-(--r-sm)" onClick={()=>void copyText(report).then(()=>toast("已复制，请检查后再分享","ok"))}>复制</button></div></div></Modal>
  </div>;
}
