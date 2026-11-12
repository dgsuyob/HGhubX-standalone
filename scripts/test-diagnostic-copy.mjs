import assert from "node:assert/strict";
import { mkdir, writeFile } from "node:fs/promises";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { createServer } from "vite";
import { chromium } from "playwright-core";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const output = join(root, "test-results", "diagnostic-copy", new Date().toISOString().replace(/[:.]/g, "-"));
await mkdir(output, { recursive: true });
const report = { environment: "Real Settings and clipboard helper; mocked Tauri IPC; not native iOS verification", passed: [], external: [] };
let vite, browser, page;
try {
  vite = await createServer({ root, server: { host: "127.0.0.1", port: 0, strictPort: false } });
  await vite.listen();
  const origin = vite.resolvedUrls.local[0];
  browser = await chromium.launch({ headless: true, executablePath: process.env.BROWSER_EXECUTABLE || undefined });
  page = await browser.newPage({ viewport: { width: 390, height: 844 }, userAgent: "Mozilla/5.0 (iPhone; CPU iPhone OS 17_1_1 like Mac OS X) AppleWebKit/605.1.15 Version/17.1 Mobile/15E148 Safari/604.1" });
  const errors = [];
  page.on("pageerror", error => errors.push(error.message));
  await page.route("**/*", route => {
    if (new URL(route.request().url()).origin === new URL(origin).origin) return route.continue();
    report.external.push(route.request().url());
    return route.abort();
  });
  await page.addInitScript(() => {
    const fixture = { delay: 0, failDiag: false, failCopy: false, emptyLog: false, failRead: false, diagCalls: 0, nativeCopies: [], webCopies: 0, calls: [] };
    Object.assign(window, {
      isTauri: true,
      diagnosticCopyFixture: fixture,
      __TAURI_INTERNALS__: { async invoke(command, args = {}) {
        if (command === "gate_state") return { base: "https://fixture.invalid", id: "fixture-library", name: "测试媒体库", adapter: "playlist_v1", activated: true, capabilities: { catalog: true, search: true, categories: true, related: true } };
        if (command === "data_call") {
          const c = args.command || {};
          const map = { boot: "app_boot", feed: "api_feed", category: "api_category", posts: "api_posts", search_hot: "api_search_hot", play: "api_play" };
          command = map[c.cmd] || c.cmd;
          args = { ...c }; delete args.cmd;
        }
        fixture.calls.push(command);
        if (command === "app_boot") return { strict_security: true, base: "https://fixture.invalid" };
        if (command === "line_state") return { base: "https://fixture.invalid", fixed: [] };
        if (command === "playback_log_status") return { enabled: false, path: "playback.log", error: null };
        if (command === "playback_log_write" || command === "debug_log") return;
        if (command === "net_diag") {
          fixture.diagCalls++;
          await new Promise(done => setTimeout(done, fixture.delay));
          if (fixture.failDiag) throw "fixture: diagnostic unavailable";
          return { verdict: "ok", summary: "连通正常", host: "fixture.invalid", base: "https://fixture.invalid", dns: { count: 1 }, tcp: { ok: true }, http: { status: 200 }, runtime: { os: "ios", ver: "fixture" } };
        }
        if (command === "playback_log_read") {
          await new Promise(done => setTimeout(done, 30));
          if (fixture.failRead) throw new Error("fixture: log unavailable");
          return fixture.emptyLog ? "" : '{"event":"fixture.playback","data":{"unicode":"诊断日志"}}\n';
        }
        if (command === "plugin:clipboard-manager|write_text") {
          if (fixture.failCopy) throw "fixture: clipboard unavailable";
          fixture.nativeCopies.push(args.text);
          return;
        }
        throw new Error(`Unexpected IPC: ${command}`);
      } },
    });
    Object.defineProperty(navigator, "clipboard", { configurable: true, value: {
      writeText: async () => { fixture.webCopies++; throw new DOMException("The request is not allowed by the user agent", "NotAllowedError"); },
    } });
  });
  const load = async () => {
    await page.goto(`${origin}tests/diagnostic-copy.html`);
    await page.waitForFunction(() => !document.querySelector('[aria-label="播放诊断日志"]').disabled);
  };
  const networkButton = () => page.getByRole("button",{name:"检查",exact:true});
  const copyButton = () => page.getByRole("dialog").getByRole("button",{name:"复制",exact:true});
  const playbackButton = () => page.getByText("复制播放诊断",{exact:true}).locator("..").locator("..").getByRole("button",{name:"复制",exact:true});

  await load();
  await page.evaluate(()=>{window.diagnosticCopyFixture.delay=250;});
  await networkButton().click();
  await page.getByText("连通正常",{exact:false}).first().waitFor();
  assert.equal(await page.evaluate(()=>window.diagnosticCopyFixture.nativeCopies.length),0,"Diagnostics must be reviewed before copying");
  await copyButton().click();
  await page.waitForFunction(()=>window.diagnosticCopyFixture.nativeCopies.length===1);
  const first=await page.evaluate(()=>window.diagnosticCopyFixture);
  assert.equal(first.webCopies,0);
  assert(first.nativeCopies[0].includes("【HGhubX 网络诊断】"));
  report.passed.push("Diagnostics remain local until the user explicitly copies; native clipboard works after async diagnostics");

  await load();
  await page.evaluate(()=>{window.diagnosticCopyFixture.failCopy=true;});
  await networkButton().click();
  await page.getByText("连通正常",{exact:false}).first().waitFor();
  await copyButton().click();
  await page.getByText("fixture: clipboard unavailable",{exact:true}).waitFor();
  assert.equal(await page.evaluate(()=>window.diagnosticCopyFixture.diagCalls),1);
  await page.screenshot({path:join(output,"report-retained.png"),fullPage:true,animations:"disabled"});
  await page.evaluate(()=>{window.diagnosticCopyFixture.failCopy=false;});
  await copyButton().click();
  await page.waitForFunction(()=>window.diagnosticCopyFixture.nativeCopies.length===1);
  assert.equal(await page.evaluate(()=>window.diagnosticCopyFixture.diagCalls),1);
  report.passed.push("Clipboard errors retain the report, expose an error and allow retry without rerunning diagnostics");

  await load();
  await page.evaluate(()=>{window.diagnosticCopyFixture.failDiag=true;});
  await networkButton().click();
  await page.getByText("fixture: diagnostic unavailable",{exact:true}).waitFor();
  assert.equal(await page.evaluate(()=>window.diagnosticCopyFixture.nativeCopies.length),0);
  report.passed.push("Failed diagnostics never report a successful copy");

  await load();await playbackButton().click();
  await page.waitForFunction(()=>window.diagnosticCopyFixture.nativeCopies.length===1);
  assert.equal(await page.evaluate(()=>window.diagnosticCopyFixture.nativeCopies[0]),'{"event":"fixture.playback","data":{"unicode":"诊断日志"}}\n');
  assert.equal(await page.evaluate(()=>window.diagnosticCopyFixture.webCopies),0);
  for(const failure of ["emptyLog","failRead"]){
    await load();await page.evaluate(key=>{window.diagnosticCopyFixture[key]=true;},failure);await playbackButton().click();
    await page.getByText(failure==="emptyLog"?"尚无诊断日志，请开启后复现问题":"fixture: log unavailable",{exact:true}).waitFor();
    assert.equal(await page.evaluate(()=>window.diagnosticCopyFixture.nativeCopies.length),0);
  }
  report.passed.push("Playback logs preserve Unicode and newlines; empty or unreadable logs are not copied");

  await load();
  await page.evaluate(()=>{window.diagnosticCopyFixture.failCopy=true;});await playbackButton().click();
  await page.getByText("fixture: clipboard unavailable",{exact:true}).waitFor();
  assert.equal(await page.getByText("诊断已复制，请检查后再分享",{exact:true}).count(),0);
  report.passed.push("Playback clipboard errors never produce success notices");

  assert.deepEqual(errors, []);
  assert.deepEqual(report.external, []);
  console.log(JSON.stringify(report, null, 2));
} catch (error) {
  report.error = String(error.stack || error);
  await page?.screenshot({ path: join(output, "failure.png"), fullPage: true }).catch(() => {});
  console.error(report.error);
  process.exitCode = 1;
} finally {
  await writeFile(join(output, "report.json"), JSON.stringify(report, null, 2));
  await browser?.close();
  await vite?.close();
  console.log(`Artifacts: ${output}`);
}
