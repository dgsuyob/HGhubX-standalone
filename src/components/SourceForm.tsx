import { useRef, useState, type FormEvent } from "react";
import { api, isMobile } from "../api";
import { useStore } from "../store";
import type { GateState } from "../types";

export default function SourceForm({ onDone }: { onDone?: () => void }) {
  const { activateGate, selectSource, gateBusy, gateError } = useStore();
  const [address, setAddress] = useState("");
  const [allowPrivate, setAllowPrivate] = useState(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const file = useRef<HTMLInputElement>(null);
  const run = async (action: () => Promise<GateState | null | undefined>) => {
    if (busy || gateBusy) return;
    setBusy(true); setError("");
    try { const state = await action(); if (state) { selectSource(state); onDone?.(); } }
    catch (e) { setError(e instanceof Error ? e.message : String(e)); }
    finally { setBusy(false); }
  };
  const connect = async (event: FormEvent) => {
    event.preventDefault();
    await run(async () => { await activateGate(address, allowPrivate); onDone?.(); return undefined; });
  };
  return <div>
    <form onSubmit={connect} className="space-y-3">
      <label className="block text-[12px] text-mut" htmlFor="library-address">媒体清单地址</label>
      <input id="library-address" value={address} onChange={e => setAddress(e.target.value)} disabled={busy || gateBusy}
        placeholder="https://your-server/library.json" autoCapitalize="none" autoCorrect="off" spellCheck={false}
        className="w-full h-12 rounded-(--r-md) bg-s2 border border-s4 px-3.5 text-[14px] outline-none focus:border-primary" />
      <label className="flex items-start gap-2 text-[11px] leading-relaxed text-mut">
        <input type="checkbox" checked={allowPrivate} onChange={e => setAllowPrivate(e.target.checked)} disabled={busy || gateBusy} className="mt-1" />
        允许访问局域网，仅用于你信任的自建媒体库
      </label>
      <button disabled={!address.trim() || busy || gateBusy} className="w-full h-11 rounded-(--r-md) bg-primary text-on-primary font-semibold text-[13px] pressable disabled:opacity-50">
        {busy || gateBusy ? "正在连接…" : "连接媒体库"}
      </button>
    </form>
    <div className="grid grid-cols-2 gap-2.5 mt-3">
      <button disabled={busy || gateBusy} onClick={() => file.current?.click()} className="h-10 rounded-(--r-md) bg-s3 text-on text-[12px] pressable disabled:opacity-50">导入清单文件</button>
      {!isMobile ? <button disabled={busy || gateBusy} onClick={() => void run(api.sourceOpenFiles)} className="h-10 rounded-(--r-md) bg-s3 text-on text-[12px] pressable disabled:opacity-50">添加本机视频</button>
        : <button disabled={busy || gateBusy} onClick={() => void run(api.sourceOpenDemo)} className="h-10 rounded-(--r-md) bg-s3 text-on text-[12px] pressable disabled:opacity-50">体验本地示例</button>}
    </div>
    {!isMobile && <button disabled={busy || gateBusy} onClick={() => void run(api.sourceOpenDemo)} className="w-full mt-4 text-primary text-[12px] pressable disabled:opacity-50">先体验本地示例</button>}
    <input ref={file} type="file" accept="application/json,.json" className="hidden" aria-label="选择媒体清单文件" onChange={event => {
      const selected = event.target.files?.[0]; event.target.value = "";
      if (selected) void run(async () => {
        if (selected.size > 2 * 1024 * 1024) throw new Error("清单文件不能超过 2 MiB");
        return api.sourceImportManifest(await selected.text(), allowPrivate);
      });
    }} />
    {(error || gateError) && <div role="alert" className="mt-3 rounded-(--r-sm) bg-danger/10 p-3 text-danger text-[12px] break-words">{error || gateError}</div>}
    <p className="mt-5 text-[11px] leading-relaxed text-mut">连接你有权访问的媒体清单。示例完全在本机播放；收藏和记录按媒体库分别保存。</p>
  </div>;
}
