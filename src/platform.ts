import { invoke } from "@tauri-apps/api/core";
import type {
  GateState,
  UpdateInfo,
} from "./types";

/** 壳层命令门面：只放非业务命令，业务数据统一走 data/client.ts。 */

/** 写一行到应用数据目录 debug.log（用户反馈「打不开」时要的就是它）。 */
export const logLine = (msg: string) => invoke("debug_log", { msg }).catch(() => {});

export const gateState = () => invoke<GateState>("gate_state");
export const gateOpen = (address: string, allowPrivateNetwork = false) => invoke<GateState>("gate_open", { address, allowPrivateNetwork });
export const sourceImportManifest = (text: string, allowPrivateNetwork = false) => invoke<GateState>("source_import_manifest", { text, allowPrivateNetwork });
export const sourceOpenFiles = () => invoke<GateState | null>("source_open_files");
export const sourceOpenDemo = () => invoke<GateState>("source_open_demo");
export const sourceReload = () => invoke<GateState>("source_reload");
export const gateReset = () => invoke<GateState>("gate_reset");

export const securitySet = (strict: boolean) => invoke<boolean>("security_set", { strict });

export const clearCache = () => invoke<void>("cache_clear");
export const netDiag = () => invoke<Record<string, unknown>>("net_diag");
/** 系统方向请求；页面不直接 invoke，统一从门面走。 */
export const setOrientation = (landscape: boolean) => invoke<void>("set_orientation", { landscape });

/* 更新与应用控制 */
export const checkUpdate = () => invoke<UpdateInfo>("check_update");
export const openUrl = (url: string) => invoke<void>("open_url", { url });

/** iOS 文本交互开关：长按快进时避免 WKWebView 放大镜。 */
export const setTextInteraction = (enabled: boolean) =>
  invoke<void>("set_text_interaction", { enabled });

/** 安卓硬件返回键走到桌面后退出应用。 */
export const exitApp = () => invoke<void>("plugin:app|exit", { code: 0 });
