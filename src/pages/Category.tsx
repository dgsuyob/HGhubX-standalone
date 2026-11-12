import { useCallback, useEffect, useRef, useState } from "react";
import { api, fmt } from "../api";
import type { Post, VideoCard } from "../types";
import { useStore } from "../store";
import { Masonry, RowCard } from "../components/cards";
import { InplaceSearch } from "../components/InplaceSearch";
import { Empty, Img } from "../components/ui";
import { IBack, IChevR, IGlobe, IGrid, ISearch, ITrophy } from "../icons";

export default function Category({wide,active}:{wide:boolean;active:boolean}) {
  const {push,gate}=useStore();
  const [tags,setTags]=useState<string[]>([]);
  const [error,setError]=useState("");
  const [loading,setLoading]=useState(false);
  const load=useCallback(async()=>{setLoading(true);setError("");try{const data=await api.tags();setTags((data.categories||[]).flatMap(c=>c.tags.map(t=>t.name)));}catch(e){setError((e as Error).message);}finally{setLoading(false);}},[]);
  useEffect(()=>{if(active&&gate.capabilities?.categories)void load();},[active,gate.id,load]);
  return <div className="absolute inset-0 bg-bg flex flex-col">
    <header className="px-5 pb-4" style={{paddingTop:"calc(var(--sat, 0px) + 16px)"}}><h1 className="text-[24px] font-bold text-on">分类</h1><p className="mt-2 text-[12px] text-mut">来自 {gate.name || "当前媒体库"} 的标签</p></header>
    <div className={`flex-1 scroll-y p-4 pb-24 ${wide?"max-w-[980px] w-full mx-auto":""}`}>
      {loading?<div className="flex justify-center py-12"><div className="w-7 h-7 spinner"/></div>:error?<Empty icon={<IGrid/>} title="分类加载失败" sub={error} action="重试" onAction={()=>void load()}/>:tags.length?<div className="grid grid-cols-2 md:grid-cols-3 gap-3">{tags.map(tag=><button key={tag} onClick={()=>push({kind:"channel",cate:tag,name:tag})} className="bg-s2 rounded-(--r-md) p-5 text-left text-on pressable"><IGrid size={22} className="text-primary mb-4"/><div className="text-[15px] font-semibold">{tag}</div><div className="text-[11px] text-mut mt-1">浏览媒体</div></button>)}</div>:<Empty icon={<IGrid/>} title="这个媒体库还没有标签" sub="可以在清单中为媒体添加标签"/>}
    </div>
  </div>;
}

/* ================= 频道详情页 ================= */
export function ChannelPage({ cate, name, onBack, wide }: { cate: string; name: string; onBack: () => void; wide: boolean }) {
  const { push } = useStore();
  const [searching, setSearching] = useState(false);
  const [sort, setSort] = useState<"最新" | "最热" | "高分">("最新");
  const [pageItems, setPageItems] = useState<VideoCard[][]>([]);
  const items = pageItems.flat();
  const [page, setPage] = useState(0);
  const [pages, setPages] = useState(1);
  const [loading, setLoading] = useState(false);
  const [err, setErr] = useState<string | null>(null);
  const busy = useRef(false);
  const sentinel = useRef<HTMLDivElement>(null);

  const loadMore = async () => {
    if (busy.current || page >= pages) return;
    busy.current = true;
    setLoading(true);
    try {
      const next = page + 1;
      const res = await api.category(cate, next, 20);
      setPageItems((prev) => {
        const seen = new Set(prev.flat().map((x) => x.id));
        return [...prev, (res?.items || []).filter((x) => !seen.has(x.id))];
      });
      setPage(next);
      setPages(res?.pagination?.pages || next);
      setErr(null);
    } catch (e) {
      setErr((e as Error).message || "加载失败");
    } finally {
      busy.current = false;
      setLoading(false);
    }
  };

  useEffect(() => { loadMore(); /* eslint-disable-next-line */ }, []);
  useEffect(() => {
    const el = sentinel.current;
    if (!el) return;
    const io = new IntersectionObserver((es) => { if (es[0].isIntersecting) loadMore(); }, { rootMargin: "600px" });
    io.observe(el);
    return () => io.disconnect();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [page, pages]);

  // 每个服务端分页内部排序，分页之间保持追加顺序：
  // 翻页时新一页不会因为 id/hot/score 更高而插到已看内容上方，避免“加载更多像刷新”。
  const sorted = pageItems.flatMap((list) => [...list].sort((a, b) => {
    if (sort === "最热") return (b.hot || 0) - (a.hot || 0);
    if (sort === "高分") return (b.score || 0) - (a.score || 0);
    return b.id - a.id;
  }));

  return (
    <div className="absolute inset-0 bg-bg flex flex-col">
      <div className="shrink-0 bg-s1" style={{ paddingTop: "var(--sat, 0px)" }}>
        <div className="px-3 pt-2 pb-3 flex items-center gap-2.5">
          <button onClick={onBack} className="w-10 h-10 rounded-full flex items-center justify-center text-on hover:bg-s3 pressable"><IBack size={20} /></button>
          <span className="w-[40px] h-[40px] rounded-(--r-sm) bg-pc text-on-pc flex items-center justify-center"><IGrid size={21} strokeWidth={1.9} /></span>
          <div className="flex-1 min-w-0">
            <div className="text-[17px] font-bold text-on leading-tight">{name}</div>
            <div className="text-[10.5px] text-mut">共 {items.length} 部 · 已加载 {page}/{Math.max(pages, 1)} 页</div>
          </div>
          <button onClick={() => setSearching(true)} aria-label="搜索短剧" className="w-9 h-9 shrink-0 rounded-full flex items-center justify-center text-on hover:bg-s3 pressable"><ISearch size={19} /></button>
        </div>
        <div className="px-4 pb-3 flex gap-2">
          {(["最新", "最热", "高分"] as const).map((s) => (
            <button key={s} onClick={() => setSort(s)}
              className={`px-4 h-8 rounded-(--r-ctl) text-[12px] font-semibold pressable ${sort === s ? "tonal text-on-sec" : "bg-s3 text-mut"}`}
              style={sort === s ? { background: "var(--p-sec)", color: "var(--p-primary)" } : undefined}>{s}</button>
          ))}
        </div>
      </div>
      <div className="flex-1 scroll-y px-4 pt-3 pb-8">
        {sorted.length ? (
          <Masonry videos={sorted} wide={wide} onOpen={(v) => push({ kind: "feed", videoIds: sorted.map((x) => x.id), index: sorted.findIndex((x) => x.id === v.id), title: name })} />
        ) : err ? (
          <Empty icon={<IGrid size={26} />} title="加载失败" sub={err} action="重试" onAction={loadMore} />
        ) : (
          <div className="flex justify-center py-10"><div className="w-8 h-8 spinner" /></div>
        )}
        <div ref={sentinel} className="h-10" />
        {loading && <div className="flex justify-center py-3"><div className="w-6 h-6 spinner" /></div>}
        {!loading && page >= pages && sorted.length > 0 && (
          <div className="text-center text-[10.5px] text-dim py-3">— 已经到底啦 —</div>
        )}
      </div>
      {searching && <InplaceSearch kind="video" wide={wide} onClose={() => setSearching(false)} onOpenVideo={(v, list) => push({ kind: "feed", videoIds: list.map((x) => x.id), index: list.findIndex((x) => x.id === v.id), title: `搜索 · ${name}` })} />}
    </div>
  );
}
