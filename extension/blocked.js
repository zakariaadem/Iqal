"use strict";

const api = globalThis.browser ?? globalThis.chrome;
const msg = (key, subs) => api.i18n.getMessage(key, subs) || key;
const params = new URLSearchParams(location.search);

document.documentElement.lang = api.i18n.getUILanguage();
document.documentElement.dir = msg("@@bidi_dir");
document.title = msg("blockedTitle");

document.getElementById("title").textContent = msg("blockedTitle");
const set = params.get("set");
document.getElementById("by").textContent = set ? msg("blockedBy", [set]) : "";

const url = params.get("url") ?? "";
let shown = url;
try {
  const u = new URL(url);
  shown = u.host + (u.pathname === "/" ? "" : u.pathname);
} catch (e) {
  // keep the raw text
}
document.getElementById("url").textContent = shown;
document.getElementById("reason").textContent = params.get("reason") ?? "";
