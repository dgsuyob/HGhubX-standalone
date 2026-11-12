import { APP_NAME, APP_DESC } from "../branding";
import { isMobile } from "../api";
import { Logo } from "../icons";
import SourceForm from "./SourceForm";
import { DesktopTitleBar } from "./DesktopChrome";
export default function SourceGate() {
  return <div className="absolute inset-0 bg-canvas text-on flex flex-col">
    {!isMobile && <DesktopTitleBar collapsed onToggle={() => {}} />}
    <main className="flex-1 overflow-y-auto flex items-center justify-center px-6 py-10">
      <div className="w-full max-w-[420px] a-rise">
        <div className="flex items-center gap-3 mb-9"><Logo size={54}/><div><h1 className="text-[27px] font-bold tracking-tight">{APP_NAME}</h1><p className="text-[12px] text-mut mt-1">{APP_DESC}</p></div></div>
        <h2 className="text-[21px] font-semibold">从你的媒体库开始</h2>
        <p className="text-[12px] text-mut mt-2 mb-6 leading-relaxed">连接自己的媒体清单，或导入本机文件，接着看你喜欢的内容。</p>
        <SourceForm />
      </div>
    </main>
  </div>;
}
