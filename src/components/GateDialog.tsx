import { useStore } from "../store";
import { Modal } from "./ui";
import SourceForm from "./SourceForm";
export default function GateDialog({open,onClose}:{open:boolean;onClose:()=>void}) {
  const {gate}=useStore();
  return <Modal open={open} onClose={onClose} width={440}><div className="p-6">
    <div className="flex items-start gap-3 mb-5"><div className="flex-1"><h2 className="text-[18px] text-on font-semibold">选择媒体库</h2><p className="text-[11px] text-mut mt-1">当前：{gate.name || "未连接"}。不同媒体库的收藏与记录分别保存。</p></div><button onClick={onClose} className="text-mut text-[12px] pressable">关闭</button></div>
    <SourceForm onDone={onClose}/>
  </div></Modal>;
}
