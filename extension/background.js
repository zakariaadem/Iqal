// Al-Dir browser extension.
//
// Asks the Al-Dir service (through the native host) about every page the
// user opens, and replaces blocked pages with a block page. Hides parts of
// pages for partial blocks. All decisions are made by the service; this
// script only reports URLs and applies the answers.

"use strict";

const api = globalThis.browser ?? globalThis.chrome;
const HOST = "org.aldir.host";
const CHECK_TIMEOUT_MS = 3000;
const RECONNECT_MS = 2000;

let port = null;
let nextId = 1;
const pending = new Map();

function browserName() {
  if (typeof globalThis.browser !== "undefined" && api.runtime.getBrowserInfo) return "firefox";
  const brands = navigator.userAgentData?.brands?.map((b) => b.brand) ?? [];
  if (brands.includes("Microsoft Edge")) return "edge";
  if (brands.includes("Brave")) return "brave";
  if (brands.includes("Google Chrome")) return "chrome";
  return "chromium";
}

function connect() {
  try {
    port = api.runtime.connectNative(HOST);
  } catch (e) {
    port = null;
    setTimeout(connect, RECONNECT_MS);
    return;
  }
  port.onMessage.addListener(onMessage);
  port.onDisconnect.addListener(() => {
    port = null;
    for (const resolve of pending.values()) resolve({ blocked: false });
    pending.clear();
    setTimeout(connect, RECONNECT_MS);
  });
  port.postMessage({ type: "hello", browser: browserName(), version: api.runtime.getManifest().version });
}

function onMessage(msg) {
  if (msg.type === "result") {
    const resolve = pending.get(msg.id);
    if (resolve) {
      pending.delete(msg.id);
      resolve(msg);
    }
  } else if (msg.type === "changed") {
    recheckAllTabs();
  }
}

function check(url) {
  if (!port) return Promise.resolve({ blocked: false });
  return new Promise((resolve) => {
    const id = nextId++;
    pending.set(id, resolve);
    port.postMessage({ type: "check", id, url });
    setTimeout(() => {
      if (pending.delete(id)) resolve({ blocked: false });
    }, CHECK_TIMEOUT_MS);
  });
}

const isWeb = (url) => /^https?:\/\//i.test(url ?? "");

function blockPage(url, result) {
  const params = new URLSearchParams({ url, set: result.set ?? "", reason: result.reason ?? "" });
  return api.runtime.getURL("blocked.html") + "?" + params.toString();
}

async function hide(tabId, selectors) {
  if (!selectors?.length) return;
  const css = selectors.join(",\n") + " { display: none !important; }";
  try {
    if (api.scripting?.insertCSS) {
      await api.scripting.insertCSS({ target: { tabId, allFrames: true }, css });
    } else {
      await api.tabs.insertCSS(tabId, { code: css, allFrames: true, runAt: "document_start" });
    }
  } catch (e) {
    // Tab closed or not injectable (browser pages); nothing to hide.
  }
}

// `full`: also apply partial-block styles (only possible once the new
// page has been committed to the tab).
async function handle(tabId, url, full) {
  if (tabId < 0 || !isWeb(url)) return;
  const result = await check(url);
  if (result.blocked) {
    api.tabs.update(tabId, { url: blockPage(url, result) }).catch(() => {});
  } else if (full) {
    hide(tabId, result.hide);
  }
}

async function recheckAllTabs() {
  const tabs = await api.tabs.query({});
  for (const tab of tabs) handle(tab.id, tab.url, true);
}

// Earliest moment: stop the page before it loads.
api.webNavigation.onBeforeNavigate.addListener((d) => {
  if (d.frameId === 0) handle(d.tabId, d.url, false);
});
// Page committed, or changed without a reload (YouTube, Reddit, ...).
for (const event of ["onCommitted", "onHistoryStateUpdated", "onReferenceFragmentUpdated"]) {
  api.webNavigation[event].addListener((d) => {
    if (d.frameId === 0) handle(d.tabId, d.url, true);
  });
}

// Every 15 s, report the page in front of the user (only while the browser
// window has focus), so the service can count time allowances.
setInterval(async () => {
  if (!port) return;
  try {
    const win = await api.windows.getLastFocused();
    if (!win || !win.focused) return;
    const [tab] = await api.tabs.query({ active: true, windowId: win.id });
    if (tab && isWeb(tab.url)) port.postMessage({ type: "active", url: tab.url });
  } catch (e) {
    // No window (browser starting or closing).
  }
}, 15000);

connect();
recheckAllTabs();
