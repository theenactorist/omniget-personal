import { extractCookiesForPlatform } from "./cookies.js";
import { detectSupportedMediaUrl } from "./detect.js";
import { createActionFeedbackController } from "./action-feedback.js";
import { registerSnifferListeners, getMediaCount, getMediaCountForPage, getDetectedMedia, getDetectedMediaForUrl, getPageKeyForTab, recordDetectedMedia, restoreMedia } from "./media-sniffer.js";
import { summarizeCookies } from "./cookie-summary.js";
import { loadSnifferState, isSnifferEnabled, setSnifferEnabled } from "./sniffer-toggle.js";
import {
  loadDeepSearchState,
  isDeepSearchEnabled,
  isDeepSearchSupported,
  setDeepSearchEnabled,
  shouldSkipDeepSearch,
} from "./deep-search-toggle.js";
import { loadH264State, isH264Enabled, isBlock60, setH264 } from "./h264-toggle.js";
import { registerContextMenu, getContextMenuId } from "./context-menu.js";
import { openOmnigetScheme } from "./send-via-scheme.js";
import { isSyntheticManifestUrl } from "./synthetic-manifest.js";
import { sendViaBridge } from "./bridge-client.js";

// Retire a polling alarm left by an earlier extension version. Pairing now
// requires the token copied by the user from the desktop app.
chrome.alarms?.clear?.("omniget-autopair")?.catch(() => {});

const INSTALL_URL = chrome.runtime.getURL("pages/options.html#install-help");
const PROTOCOL_VERSION = 1;

function getIconPath(iconSet) {
  return {
    16: chrome.runtime.getURL(iconSet[16]),
    24: chrome.runtime.getURL(iconSet[24]),
    32: chrome.runtime.getURL(iconSet[32]),
    48: chrome.runtime.getURL(iconSet[48]),
  };
}

const ACTIVE_ICON_PATHS = Object.freeze({
  16: "icons/active-16.png",
  24: "icons/active-24.png",
  32: "icons/active-32.png",
  48: "icons/active-48.png",
});

const INACTIVE_ICON_PATHS = Object.freeze({
  16: "icons/inactive-16.png",
  24: "icons/inactive-24.png",
  32: "icons/inactive-32.png",
  48: "icons/inactive-48.png",
});

const actionFeedback = createActionFeedbackController({
  setBadgeText: (details) => chrome.action.setBadgeText(details),
  setBadgeBackgroundColor: (details) => chrome.action.setBadgeBackgroundColor(details),
});

// Registered unconditionally, synchronously, before anything awaits: see the
// note on registerSnifferListeners. The stored enable flag is consulted inside
// the handlers, so a disabled sniffer costs one boolean check per request.
registerSnifferListeners(onMediaDetected);

loadSnifferState().catch(() => {});
loadDeepSearchState().catch(() => {});
loadH264State().catch(() => {});
restoreMedia().catch(() => {});

chrome.runtime.onInstalled.addListener(async (details) => {
  registerContextMenu();
  refreshActiveTab().catch(() => {});
  // Surface the pairing page on any install/update *if the user hasn't
  // already paired this browser*. Reloading an unpacked extension fires
  // `update`, not `install`, so gating only on `install` would silently
  // skip the onboarding flow for dev builds and users coming from a
  // pre-bridge OmniGet version.
  if (typeof chrome.runtime.openOptionsPage !== "function") return;
  try {
    const stored = await chrome.storage.local.get("bridge_token");
    const token = typeof stored?.bridge_token === "string" ? stored.bridge_token.trim() : "";
    if (!token) {
      chrome.runtime.openOptionsPage().catch(() => {});
    }
  } catch {
    // storage unavailable — fall back to the previous behaviour and only
    // open on a real install.
    if (details?.reason === "install") {
      chrome.runtime.openOptionsPage().catch(() => {});
    }
  }
});

chrome.contextMenus.onClicked.addListener(async (info, tab) => {
  if (info.menuItemId !== getContextMenuId()) return;

  const url = info.linkUrl || info.srcUrl;
  if (!url) return;

  const result = await handleSendToApp({
    type: "sendToOmniGet",
    url,
    platform: "generic",
    referer: tab?.url || "",
  });

  if (result.ok) {
    actionFeedback.showSuccessBadge(tab?.id);
  }
});

chrome.runtime.onStartup.addListener(() => {
  refreshActiveTab().catch(() => {});
});

if (chrome.commands && chrome.commands.onCommand) {
  chrome.commands.onCommand.addListener(async (command) => {
    if (command !== "send-to-omniget") return;
    try {
      const [tab] = await chrome.tabs.query({ active: true, currentWindow: true });
      if (!tab?.url) return;
      const detected = detectSupportedMediaUrl(tab.url);
      if (detected?.supported) {
        const result = await handleSendToApp({
          type: "sendToOmniGet",
          url: tab.url,
          platform: detected.platform,
          referer: tab.url,
        });
        if (result?.ok && tab.id !== undefined) {
          actionFeedback.showSuccessBadge(tab.id);
        }
        return;
      }
      if (chrome.action && typeof chrome.action.openPopup === "function") {
        try { await chrome.action.openPopup(); } catch {}
      }
    } catch (error) {
      console.error("[OmniGet] command handler failed:", error);
    }
  });
}

chrome.tabs.onActivated.addListener(() => {
  refreshActiveTab().catch(() => {});
});

chrome.tabs.onUpdated.addListener((tabId, changeInfo, tab) => {
  if (!changeInfo.url && !changeInfo.status) {
    return;
  }
  if (!tab?.url) {
    return;
  }
  refreshTabAction(tabId, tab).catch((error) => {
    console.error("[OmniGet] Failed to refresh tab action:", error);
  });
});

chrome.windows.onFocusChanged.addListener((windowId) => {
  if (windowId !== chrome.windows.WINDOW_ID_NONE) {
    refreshActiveTab().catch(() => {});
  }
});

chrome.runtime.onMessage.addListener((msg, sender, sendResponse) => {
  if (msg.type === "getDetectedMedia") {
    chrome.tabs.query({ active: true, currentWindow: true }, (tabs) => {
      const tabId = tabs[0]?.id;
      const pageUrl = tabs[0]?.url;
      if (!tabId) { sendResponse({ media: [], snifferEnabled: isSnifferEnabled() }); return; }

      const media = pageUrl
        ? getDetectedMediaForUrl(pageUrl)
        : getDetectedMedia(tabId);
      const list = Array.from(media.values()).sort((a, b) => b.detectedAt - a.detectedAt);

      const pageDetected = detectSupportedMediaUrl(pageUrl);

      sendResponse({
        media: list,
        pageDetected,
        snifferEnabled: isSnifferEnabled(),
        tabUrl: pageUrl,
      });
    });
    return true;
  }

  if (msg.type === "getH264") {
    sendResponse({ enabled: isH264Enabled(), block60: isBlock60() });
    return false;
  }

  if (msg.type === "toggleH264") {
    setH264(msg.enabled, msg.block60).then((r) => sendResponse(r)).catch(() => sendResponse({ enabled: isH264Enabled(), block60: isBlock60() }));
    return true;
  }

  if (msg.type === "toggleSniffer") {
    setSnifferEnabled(msg.enabled).then((result) => {
      const effective = isSnifferEnabled();
      sendResponse({
        ok: result?.ok !== false,
        enabled: effective,
        reason: result?.reason,
      });
    });
    return true;
  }

  if (msg.type === "sendToOmniGet") {
    handleSendToApp(msg).then(sendResponse);
    return true;
  }

  // Deep search runs in the page and finds playlists the network layer never
  // sees as media. The manifest text itself stays in the content script (it can
  // be hundreds of KB); we only fetch it at send time, from the tab that has it.
  if (msg.type === "deep-search-media") {
    const tabId = sender?.tab?.id;
    if (typeof tabId !== "number") return false;
    if (shouldSkipDeepSearch(msg.pageUrl)) return false;
    const entry = {
      url: msg.url,
      contentType: msg.format === "dash" ? "application/dash+xml" : "application/x-mpegurl",
      contentLength: 0,
      mediaType: msg.format === "dash" ? "dash" : "hls",
      sizeText: "",
      detectedAt: Date.now(),
      tabId,
      requestHeaders: [],
      responseHeaders: [],
      source: "deep-search",
      hasManifest: Boolean(msg.hasManifest),
      // Which frame holds the text: an iframe player keeps its own copy, and
      // asking the whole tab would take whichever frame answered first.
      frameId: typeof sender?.frameId === "number" ? sender.frameId : 0,
    };
    recordDetectedMedia(tabId, entry).then((recorded) => {
      if (recorded) onMediaDetected(tabId, entry);
    });
    return false;
  }

  if (msg.type === "getDeepSearch") {
    isDeepSearchSupported()
      .then((supported) => sendResponse({ enabled: isDeepSearchEnabled(), supported }))
      .catch(() => sendResponse({ enabled: false, supported: false }));
    return true;
  }

  if (msg.type === "toggleDeepSearch") {
    setDeepSearchEnabled(msg.enabled)
      .then((result) => sendResponse({
        ok: result?.ok !== false,
        enabled: isDeepSearchEnabled(),
        reason: result?.reason,
      }))
      .catch(() => sendResponse({ ok: false, enabled: isDeepSearchEnabled() }));
    return true;
  }
});

// The text lives in the deep-search bridge of the tab that captured it. A tab
// that has since navigated away answers nothing, which is fine: the app falls
// back to fetching the URL, and only a blob-backed playlist is truly lost.
async function fetchManifestText(tabId, frameId, url) {
  if (typeof tabId !== "number") return null;
  try {
    const options = typeof frameId === "number" ? { frameId } : undefined;
    const response = await chrome.tabs.sendMessage(tabId, { type: "getManifestText", url }, options);
    const text = response?.text;
    return typeof text === "string" && text !== "" ? text : null;
  } catch {
    return null;
  }
}

function onMediaDetected(tabId, _entry) {
  if (!isSnifferEnabled()) return;
  updateBadge(tabId);
  const pageKey = getPageKeyForTab(tabId);
  if (!pageKey) return;
  const count = getMediaCountForPage(pageKey);
  chrome.runtime.sendMessage({
    type: "media-detected",
    pageKey,
    count,
  }).catch(() => {});
}

function updateBadge(tabId) {
  const count = getMediaCount(tabId);
  chrome.action.setBadgeText({
    tabId,
    text: count > 0 ? String(count) : "",
  }).catch(() => {});
  chrome.action.setBadgeBackgroundColor({
    tabId,
    color: "#F04E23",
  }).catch(() => {});
}

async function handleSendToApp(msg) {
  const url = msg.url;
  const platform = msg.platform || "generic";

  let pageTitle = "";
  let pageThumbnail = "";
  try {
    const [tab] = await chrome.tabs.query({ active: true, currentWindow: true });
    pageTitle = tab?.title || "";
    pageThumbnail = tab?.favIconUrl || "";
  } catch {}

  let cookies = null;
  try {
    const platformCookies = await extractCookiesForPlatform(platform);
    if (platformCookies && platformCookies.length > 0) {
      cookies = platformCookies;
    } else {
      const cookieMap = new Map();

      const cdnCookies = await chrome.cookies.getAll({ url });
      for (const c of cdnCookies) {
        cookieMap.set(`${c.domain}:${c.name}`, c);
      }

      if (msg.referer) {
        try {
          const pageCookies = await chrome.cookies.getAll({ url: msg.referer });
          for (const c of pageCookies) {
            cookieMap.set(`${c.domain}:${c.name}`, c);
          }
        } catch {}
      }

      if (cookieMap.size > 0) {
        cookies = [...cookieMap.values()].map(c => ({
          domain: c.domain,
          httpOnly: c.httpOnly,
          path: c.path,
          secure: c.secure,
          expires: c.expirationDate ? Math.floor(c.expirationDate) : 0,
          name: c.name,
          value: c.value,
          hostOnly: c.hostOnly,
          sameSite: c.sameSite,
        }));
      }
    }
  } catch {}

  const message = { type: "enqueue", url, protocolVersion: PROTOCOL_VERSION };
  if (cookies) message.cookies = cookies;
  if (msg.referer) message.referer = msg.referer;
  if (msg.title) message.title = msg.title;
  else if (pageTitle) message.title = pageTitle;
  if (msg.thumbnail) message.thumbnail = msg.thumbnail;
  else if (pageThumbnail) message.thumbnail = pageThumbnail;
  if (msg.mediaType) message.mediaType = msg.mediaType;
  if (msg.hasManifest) {
    let tabId = msg.tabId;
    if (typeof tabId !== "number") {
      try {
        const [tab] = await chrome.tabs.query({ active: true, currentWindow: true });
        tabId = tab?.id;
      } catch {}
    }
    const manifestText = await fetchManifestText(tabId, msg.frameId, url);
    if (manifestText) {
      message.manifestText = manifestText;
    } else if (isSyntheticManifestUrl(url)) {
      // The text lived in the page and the page is gone — a reload, a closed
      // tab, an evicted entry. A synthetic URL is an identity, not an address,
      // so there is nothing left to download and no fallback to try. Say so
      // instead of queueing a URL that can only fail at DNS.
      return { ok: false, reason: "manifest_gone" };
    }
  }
  if (msg.contentType) message.contentType = msg.contentType;
  if (msg.headers) message.headers = msg.headers;
  if (typeof msg.openApp === "boolean") message.openApp = msg.openApp;
  message.pageUrl = msg.referer || "";
  message.userAgent = navigator.userAgent;

  try {
    await chrome.storage.local.set({
      last_download_metadata: {
        url,
        referer: msg.referer || "",
        headers: msg.headers || {},
        cookies: cookies || [],
        userAgent: navigator.userAgent,
        timestamp: Date.now(),
      },
    }).catch(() => {});
  } catch {}

  const cookieSummary = summarizeCookies(cookies);

  // Primary path: localhost HTTP bridge (no extension-ID dependency, full
  // cookie + metadata payload).
  const bridgeResult = await sendViaBridge(message);
  if (bridgeResult?.ok) {
    return { ok: true, viaBridge: true, cookieSummary };
  }

  // Fallback: omniget-personal:// scheme handler. The desktop app is launched (or
  // brought to focus) and the URL is queued, but cookies aren't forwarded
  // — the user can pair the bridge from the extension's options page to
  // get the full experience.
  const schemeResult = await openOmnigetScheme(url);
  if (schemeResult?.ok) {
    return { ok: true, viaScheme: true, cookieSummary, bridgeReason: bridgeResult?.reason };
  }
  return {
    ok: false,
    error: bridgeResult?.message || schemeResult?.message || "Origo Downloads is not reachable",
    bridgeReason: bridgeResult?.reason,
    schemeReason: schemeResult?.reason,
  };
}

async function refreshActiveTab() {
  const [tab] = await chrome.tabs.query({
    active: true,
    lastFocusedWindow: true,
  });

  if (tab?.id !== undefined) {
    await refreshTabAction(tab.id, tab);
  }
}

async function refreshTabAction(tabId, tab) {
  if (!tab?.url) {
    return;
  }

  const detected = detectSupportedMediaUrl(tab.url);
  const supported = Boolean(detected?.supported);
  const mediaCount = getMediaCount(tabId);

  try {
    const iconSet = supported ? ACTIVE_ICON_PATHS : INACTIVE_ICON_PATHS;
    await chrome.action.setIcon({ tabId, path: getIconPath(iconSet) });
  } catch (error) {
    if (isTabGoneError(error)) return;
  }

  if (mediaCount > 0) {
    updateBadge(tabId);
  } else {
    try { await actionFeedback.clearBadge(tabId); } catch {}
  }
}

function isTabGoneError(error) {
  const msg = error instanceof Error ? error.message : String(error);
  return msg.includes("No tab with id");
}

// Cookies are exported only by explicit download/capture actions. Browser
// startup, tab navigation and cookie changes never trigger credential export.
