import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import vm from "node:vm";

// Run the real event handlers with isolated browser/module boundaries. This
// exercises ambient browser events and the user action separately, without
// accessing a real browser profile, account or desktop bridge.
function backgroundHarness(browser) {
  const source = readFileSync(new URL(`../../${browser}/src/background.js`, import.meta.url), "utf8");
  const credentialReads = [];
  const exports = [];
  const network = [];
  const alarms = [];
  const cookie = { domain: ".youtube.com", name: "SID", value: "session", httpOnly: true };
  const tab = { id: 4, url: "https://www.youtube.com/watch?v=video", title: "Video" };
  const noOp = () => {};
  const asyncNoOp = async () => {};
  function event() {
    const handlers = [];
    return { addListener: fn => handlers.push(fn), async emit(...args) {
      await Promise.all(handlers.map(fn => fn(...args)));
    }, handlers };
  }
  const chrome = {
    alarms: { clear: asyncNoOp, create: (...args) => alarms.push(args), get: asyncNoOp, onAlarm: event() },
    runtime: { onInstalled: event(), onStartup: event(), onMessage: event(), getURL: x => `chrome-extension://test/${x}`, openOptionsPage: asyncNoOp },
    cookies: { onChanged: event(), getAll: async (query) => { credentialReads.push(query); return [cookie]; } },
    storage: { onChanged: event(), local: { get: async () => ({}), set: asyncNoOp } },
    contextMenus: { onClicked: event() }, commands: { onCommand: event() },
    tabs: { onUpdated: event(), onActivated: event(), query: async () => [tab] },
    windows: { onFocusChanged: event(), WINDOW_ID_NONE: -1 },
    action: { setIcon: asyncNoOp, setBadgeText: asyncNoOp, setBadgeBackgroundColor: asyncNoOp },
  };
  const context = {
    chrome, console, URL, Map, Set, navigator: { userAgent: "test-browser" },
    setTimeout: (fn) => { queueMicrotask(fn); return 1; }, clearTimeout: noOp,
    fetch: async (...args) => { network.push(args); throw new Error("unexpected network request"); },
    extractCookiesForPlatform: async (platform) => { credentialReads.push(platform); return [cookie]; },
    detectSupportedMediaUrl: () => ({ supported: true, platform: "youtube" }),
    createActionFeedbackController: () => ({ clearBadge: asyncNoOp, showSuccessBadge: noOp }),
    registerSnifferListeners: noOp, restoreMedia: asyncNoOp, getMediaCount: () => 0,
    loadSnifferState: asyncNoOp, loadDeepSearchState: asyncNoOp, loadH264State: asyncNoOp,
    registerContextMenu: noOp, getContextMenuId: () => "send",
    summarizeCookies: values => ({ count: values?.length ?? 0 }),
    sendViaBridge: async payload => { exports.push(payload); return { ok: true }; },
  };
  vm.runInNewContext(source.replace(/^import\s+[\s\S]*?;\s*/gm, ""), context, { filename: `${browser}/background.js` });
  return { chrome, credentialReads, exports, network, alarms, cookie, tab };
}

for (const browser of ["chrome", "firefox"]) {
  test(`${browser}: startup, install, browsing and cookie changes never export credentials`, async () => {
    const h = backgroundHarness(browser);
    await h.chrome.runtime.onStartup.emit();
    await h.chrome.runtime.onInstalled.emit({ reason: "install" });
    await h.chrome.tabs.onUpdated.emit(h.tab.id, { status: "complete" }, h.tab);
    await h.chrome.cookies.onChanged.emit({ cookie: h.cookie, removed: false });
    await h.chrome.alarms.onAlarm.emit({ name: "omniget-autopair" });
    await new Promise(setImmediate);
    assert.deepEqual(h.credentialReads, []);
    assert.deepEqual(h.exports, []);
    assert.deepEqual(h.network, []);
    assert.deepEqual(h.alarms, []);
  });

  test(`${browser}: explicit download still exports the selected platform's cookies`, async () => {
    const h = backgroundHarness(browser);
    const response = await new Promise(resolve => {
      for (const handler of h.chrome.runtime.onMessage.handlers) {
        handler({ type: "sendToOmniGet", url: h.tab.url, platform: "youtube", referer: h.tab.url }, {}, resolve);
      }
    });
    assert.equal(response.ok, true);
    assert.equal(response.viaBridge, true);
    assert.deepEqual(h.credentialReads, ["youtube"]);
    assert.equal(h.exports.length, 1);
    assert.equal(h.exports[0].url, h.tab.url);
    assert.equal(h.exports[0].cookies[0].value, "session");
  });
}
