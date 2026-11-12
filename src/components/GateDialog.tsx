import { useEffect, useState } from "react";
import { useStore } from "../store";
import { Modal } from "./ui";
import { ILink } from "../icons";

/** 设置里的接入地址弹窗：与首屏 SourceGate 共用同一套零校验 gate_open。 */
export default function GateDialog({ open, onClose }: { open: boolean; onClose: () => void }) {
  const { gate, activateGate, resetGate, gateBusy, gateError } = useStore();
  const [value, setValue] = useState(gate.base);

  useEffect(() => { if (open) setValue(gate.base); }, [open, gate.base]);

  const submit = async () => {
    try {
      await activateGate(value);
      onClose();
    } catch { /* gateError 已在 store 中 */ }
  };

  return (
    <Modal open={open} onClose={onClose} width={380}>
      <div className="p-5">
        <div className="flex items-center gap-2.5">
          <span className="w-8 h-8 rounded-full bg-s3 flex items-center justify-center text-primary"><ILink size={16} /></span>
          <div>
            <div className="text-[14px] font-bold text-on">接入地址</div>
            <div className="text-[10.5px] text-mut mt-0.5">地址只保存在本机，不校验格式</div>
          </div>
        </div>
        <div className={`mt-4 flex items-center gap-2 h-11 rounded-(--r-md) bg-s3 border px-3 ${gateError ? "border-danger/60" : "border-s4/60"}`}>
          <ILink size={15} className="text-mut shrink-0" />
          <input value={value} onChange={(e) => setValue(e.target.value)} aria-label="接入地址"
            autoCapitalize="none" autoCorrect="off" spellCheck={false}
            className="flex-1 min-w-0 bg-transparent outline-none text-[13px]" />
        </div>
        {gateError && <div className="text-[11px] text-danger mt-2">{gateError}</div>}
        <div className="flex gap-2 mt-4">
          <button onClick={onClose} className="flex-1 h-9 rounded-(--r-ctl) bg-s3 text-[12.5px] text-on pressable">取消</button>
          <button onClick={() => void submit()} disabled={gateBusy || !value.trim()}
            className="flex-1 h-9 rounded-(--r-ctl) bg-primary text-on-primary text-[12.5px] font-semibold pressable disabled:opacity-50">
            {gateBusy ? "连接中…" : "保存并连接"}
          </button>
        </div>
        <button onClick={() => { void resetGate().then(onClose); }}
          className="mt-3 w-full text-center text-[11px] text-mut pressable">清除接入地址</button>
      </div>
    </Modal>
  );
}
