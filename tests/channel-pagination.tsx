import React from "react";
import { createRoot } from "react-dom/client";
import { ChannelPage } from "../src/pages/Category";
import { AppProvider } from "../src/store";
import "../src/index.css";

const page1Ids = [12, 117, 2833, 323, 7628, 2181, 4466, 89, 5000, 903, 6000, 445, 123, 800, 701, 60, 1000, 3000, 20, 40];
const page2Ids = [3483, 131, 6174, 3273, 1237, 7000, 500, 9000, 52, 84, 600, 1200, 2500, 3300, 4100, 5200, 6600, 7100, 8100, 9100];

const makeVideo = (ids: number[]) => ids.map((id, index) => ({
  id,
  title: `Video ${id}`,
  cover: "",
  tags: [] as string[],
  score: (index * 13 + 5) % 101,
  hot: (index * 17 + 11) % 101,
  episode_count: 1,
  total_episodes: 1,
  is_finished: true,
  is_original: 0,
  like_count: 0,
  comment_count: 0,
  collect_count: 0,
}));

const page1 = makeVideo(page1Ids);
const page2 = makeVideo(page2Ids);
const pending: { args: Record<string, unknown>; resolve: (value: unknown) => void }[] = [];
const fixture = {
  history: [] as { cmd: string; args: Record<string, unknown> }[],
  pending,
  page1,
  page2,
  resolveNext() {
    const request = pending.shift();
    if (!request) throw new Error("no pending api_category request");
    const page = Number(request.args.page);
    request.resolve({
      items: page === 1 ? page1 : page2,
      pagination: { page, pages: 2, size: 20, total: 40 },
    });
  },
};

Object.assign(window, {
  channelPaginationFixture: fixture,
  __TAURI_INTERNALS__: {
    invoke(cmd: string, args: Record<string, unknown> = {}) {
      if (cmd === "gate_state") return Promise.resolve({ base: "https://fixture.invalid", id: "fixture-library", name: "测试媒体库", adapter: "playlist_v1", activated: true, capabilities: { catalog: true, search: true, categories: true, related: true } });
      if (cmd === "data_call") {
        const command = (args.command || {}) as Record<string, unknown>;
        const map: Record<string, string> = { boot: "app_boot", feed: "api_feed", category: "api_category", posts: "api_posts", search_hot: "api_search_hot", play: "api_play" };
        cmd = map[String(command.cmd)] || String(command.cmd);
        args = { ...command }; delete (args as Record<string, unknown>).cmd;
      }
      if (cmd === "app_boot") return Promise.resolve({ user: null, logged_in: false, base: "https://fixture.invalid" });
      if (cmd === "debug_log") return Promise.resolve();
      if (cmd === "api_category") {
        fixture.history.push({ cmd, args });
        return new Promise((resolve) => pending.push({ args, resolve }));
      }
      return Promise.reject(`Unexpected IPC: ${cmd}`);
    },
  },
});

createRoot(document.querySelector("#root")!).render(
  <React.StrictMode>
    <AppProvider>
      <div style={{ position: "relative", width: 390, height: 844 }}>
        <ChannelPage cate="travel" name="旅行记录" onBack={() => {}} wide={false} />
      </div>
    </AppProvider>
  </React.StrictMode>,
);
