import { useEffect, useRef, useState } from "react";
import { api, isMobile } from "../api";
import type { VideoCard } from "../types";
import { useStore } from "../store";
import { Masonry } from "../components/cards";
import { Empty } from "../components/ui";
import { IBack, ISearch } from "../icons";
export default function Search({active,wide}:{active:boolean;wide:boolean}) {
  const {gate,push,searchHist,addSearch,clearSearchHist}=useStore();
  const [q,setQ]=useState(""),[submitted,setSubmitted]=useState(""),[items,setItems]=useState<VideoCard[]>([]),[loading,setLoading]=useState(false),[error,setError]=useState("");
  const [page,setPage]=useState(1),[pages,setPages]=useState(1);const sequence=useRef(0);const input=useRef<HTMLInputElement>(null);
  useEffect(()=>{if(active&&!isMobile)input.current?.focus();},[active]);
  const run=async(keyword:string,next=1)=>{const query=keyword.trim();if(!query)return;const request=++sequence.current;setLoading(true);setError("");setSubmitted(query);addSearch(query);if(next===1)setItems([]);
    try{const result=await api.search(query,next,30);if(request!==sequence.current)return;setItems(old=>next===1?result.items:[...old,...result.items.filter(v=>!old.some(x=>x.id===v.id))]);setPage(next);setPages(result.pagination.pages);}catch(e){if(request===sequence.current)setError((e as Error).message);}finally{if(request===sequence.current)setLoading(false);}};
  return <div className="absolute inset-0 bg-bg flex flex-col"><header className="px-4 pb-4" style={{paddingTop:"calc(var(--sat, 0px) + 16px)"}}><h1 className="text-[23px] font-bold text-on mb-3">搜索</h1><form className="flex gap-2" onSubmit={e=>{e.preventDefault();void run(q);}}><input ref={input} aria-label="搜索媒体库" placeholder="搜索标题、描述或标签…" value={q} onChange={e=>setQ(e.target.value)} className="flex-1 min-w-0 h-11 px-4 bg-s3 rounded-(--r-md) text-on outline-none focus:ring-1 focus:ring-primary"/><button className="h-11 px-4 bg-primary text-on-primary rounded-(--r-md) pressable" disabled={!gate.capabilities?.search}>搜索</button></form><p className="text-[11px] text-mut mt-2">仅搜索 {gate.name||"当前媒体库"}</p></header>
    <main className={`flex-1 scroll-y px-4 pb-24 ${wide?"max-w-[1100px] w-full mx-auto":""}`}>
      {!submitted?<><div className="flex justify-between text-[12px] text-mut my-4"><span>搜索历史</span>{searchHist.length>0&&<button onClick={clearSearchHist}>清空</button>}</div><div className="flex flex-wrap gap-2">{searchHist.map(word=><button key={word} className="px-3 py-2 bg-s3 rounded-full text-on text-[12px]" onClick={()=>{setQ(word);void run(word);}}>{word}</button>)}</div>{!searchHist.length&&<Empty icon={<ISearch/>} title="搜索你的媒体" sub="输入标题、描述或标签，查找当前媒体库中的内容"/>}</>:<><p className="text-[12px] text-mut my-3">「{submitted}」{items.length>0?` · 已显示 ${items.length} 个条目`:""}</p>{items.length>0&&<Masonry videos={items} wide={wide} onOpen={v=>push({kind:"feed",videoIds:items.map(i=>i.id),index:items.findIndex(i=>i.id===v.id),title:`搜索 · ${submitted}`})}/>} {loading?<div className="flex justify-center py-8"><div className="w-7 h-7 spinner"/></div>:error?<Empty icon={<ISearch/>} title="搜索失败" sub={error} action="重试" onAction={()=>void run(submitted,page)}/>:!items.length?<Empty icon={<ISearch/>} title="没有找到相关内容" sub="可以换个关键词试试"/>:page<pages&&<button className="w-full h-10 my-5 bg-s3 rounded-(--r-md) text-on" onClick={()=>void run(submitted,page+1)}>加载更多</button>}</>}
    </main></div>;
}
export function RankPage({onBack}:{onBack:()=>void;wide:boolean}) {return <div className="absolute inset-0 bg-bg text-on p-5"><button onClick={onBack} className="flex gap-2 items-center"><IBack/>返回</button><Empty icon={<ISearch/>} title="当前媒体库不提供排行榜"/></div>;}
