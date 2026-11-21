// ==============================================================================
// RUSTAD WORKER (worker.js)
// Service worker managing declarative network rules, WASM cosmetics, and in-memory badge
// ==============================================================================

let wasmInstance = null;
let memoryBuffer = null;

// In-memory badge counts and distinct blocked URLs per tab
const tabCounts = new Map();
const tabBlockedUrls = new Map();

function setBadge(tabId, count) {
  if (tabId < 0) return;
  const text = count > 0 ? String(count) : "";
  chrome.action.setBadgeText({ tabId, text });
  chrome.action.setBadgeBackgroundColor({ tabId, color: "#e05666" });
  if (chrome.action.setBadgeTextColor) {
    chrome.action.setBadgeTextColor({ tabId, color: "#ffffff" });
  }
}

function incrementBadge(tabId, amount = 1) {
  if (tabId < 0) return;
  const current = (tabCounts.get(tabId) || 0) + amount;
  tabCounts.set(tabId, current);
  setBadge(tabId, current);
}

// Reset badge on tab navigation or removal
chrome.tabs.onUpdated.addListener((tabId, changeInfo) => {
  if (changeInfo.status === "loading") {
    tabCounts.delete(tabId);
    tabBlockedUrls.delete(tabId);
    setBadge(tabId, 0);
  }
});

chrome.tabs.onRemoved.addListener((tabId) => {
  tabCounts.delete(tabId);
  tabBlockedUrls.delete(tabId);
});

// Track distinct network ad blocks in real time (prevents retry loop badge spikes)
if (chrome.declarativeNetRequest.onRuleMatchedDebug) {
  chrome.declarativeNetRequest.onRuleMatchedDebug.addListener((info) => {
    if (info && info.request && typeof info.request.tabId === "number") {
      const tabId = info.request.tabId;
      if (tabId < 0) return;
      const url = info.request.url || "";
      if (!tabBlockedUrls.has(tabId)) {
        tabBlockedUrls.set(tabId, new Set());
      }
      const set = tabBlockedUrls.get(tabId);
      if (!set.has(url)) {
        set.add(url);
        incrementBadge(tabId, 1);
      }
    }
  });
}

async function loadWasm() {
  if (wasmInstance) return wasmInstance;
  try {
    const response = await fetch(chrome.runtime.getURL("rustad.wasm"));
    const bytes = await response.arrayBuffer();
    const result = await WebAssembly.instantiate(bytes, {});
    wasmInstance = result.instance;
    memoryBuffer = wasmInstance.exports.memory.buffer;
    return wasmInstance;
  } catch (err) {
    console.error("Failed to load rustad.wasm:", err);
    return null;
  }
}

// Generic scriptlet runner executed in page's MAIN world via chrome.scripting
function runGenericScriptlets(rules) {
  if (!rules || !Array.isArray(rules)) return;
  for (const rule of rules) {
    const parts = rule.split(',').map(s => s.trim());
    const fn = parts[0];
    const args = parts.slice(1);

    if (fn === 'set') {
      const path = args[0];
      let val = args[1];
      if (val === 'undefined') val = undefined;
      else if (val === 'null') val = null;
      else if (val === 'false') val = false;
      else if (val === 'true') val = true;
      else if (val === '[]') val = [];
      else if (val === '{}') val = {};
      else if (val === 'noopFunc') val = () => {};

      if (!path) continue;
      const segments = path.split('.');
      let obj = window;
      for (let i = 0; i < segments.length - 1; i++) {
        const seg = segments[i];
        if (!obj[seg] || typeof obj[seg] !== 'object') {
          obj[seg] = {};
        }
        obj = obj[seg];
      }
      const last = segments[segments.length - 1];
      try {
        Object.defineProperty(obj, last, {
          get: () => val,
          set: () => {},
          configurable: true,
          enumerable: true
        });
      } catch (_) {
        try { obj[last] = val; } catch (_) {}
      }
    } else if (fn === 'abort-on-property-read') {
      const path = args[0];
      if (!path) continue;
      const segments = path.split('.');
      let obj = window;
      for (let i = 0; i < segments.length - 1; i++) {
        const seg = segments[i];
        if (!obj[seg] || typeof obj[seg] !== 'object') {
          obj[seg] = {};
        }
        obj = obj[seg];
      }
      const last = segments[segments.length - 1];
      try {
        Object.defineProperty(obj, last, {
          get: () => { throw new ReferenceError(last + ' read aborted'); },
          set: () => {},
          configurable: true
        });
      } catch (_) {}
    } else if (fn === 'abort-on-property-write') {
      const path = args[0];
      if (!path) continue;
      const segments = path.split('.');
      let obj = window;
      for (let i = 0; i < segments.length - 1; i++) {
        const seg = segments[i];
        if (!obj[seg] || typeof obj[seg] !== 'object') {
          obj[seg] = {};
        }
        obj = obj[seg];
      }
      const last = segments[segments.length - 1];
      try {
        Object.defineProperty(obj, last, {
          get: () => undefined,
          set: () => { throw new ReferenceError(last + ' write aborted'); },
          configurable: true
        });
      } catch (_) {}
    }
  }
}

// Compute cosmetic CSS string via WASM
async function getCosmeticCss(url) {
  const instance = await loadWasm();
  if (!instance) return "";

  const inputPtr = instance.exports.rustad_input_ptr();
  const outputPtr = instance.exports.rustad_output_ptr();
  memoryBuffer = instance.exports.memory.buffer;

  const encoder = new TextEncoder();
  const encoded = encoder.encode(url);
  const len = Math.min(encoded.length, 2048);

  const inputView = new Uint8Array(memoryBuffer, inputPtr, len);
  inputView.set(encoded.subarray(0, len));

  const writtenLen = instance.exports.rustad_get_cosmetics(len);
  if (writtenLen === 0) return "";

  memoryBuffer = instance.exports.memory.buffer;
  const outputView = new Uint8Array(memoryBuffer, outputPtr, writtenLen);
  const decoder = new TextDecoder();
  return decoder.decode(outputView);
}

// Compute matching scriptlets array via WASM
async function getScriptlets(url) {
  const instance = await loadWasm();
  if (!instance || !instance.exports.rustad_get_scriptlets) return [];

  const inputPtr = instance.exports.rustad_input_ptr();
  const outputPtr = instance.exports.rustad_output_ptr();
  memoryBuffer = instance.exports.memory.buffer;

  const encoder = new TextEncoder();
  const encoded = encoder.encode(url);
  const len = Math.min(encoded.length, 2048);

  const inputView = new Uint8Array(memoryBuffer, inputPtr, len);
  inputView.set(encoded.subarray(0, len));

  const writtenLen = instance.exports.rustad_get_scriptlets(len);
  if (writtenLen === 0) return [];

  memoryBuffer = instance.exports.memory.buffer;
  const outputView = new Uint8Array(memoryBuffer, outputPtr, writtenLen);
  const decoder = new TextDecoder();
  const jsonStr = decoder.decode(outputView);
  try {
    return JSON.parse(jsonStr);
  } catch (_) {
    return [];
  }
}

// Update declarativeNetRequest ruleset state
async function setRulesetEnabled(enabled) {
  try {
    await chrome.declarativeNetRequest.updateEnabledRulesets({
      enableRulesetIds: enabled ? ["ruleset_1"] : [],
      disableRulesetIds: enabled ? [] : ["ruleset_1"]
    });
  } catch (err) {
    console.error("Failed to update ruleset:", err);
  }
}

// Message Router
chrome.runtime.onMessage.addListener((message, sender, sendResponse) => {
  if (!message || !message.action) return false;

  const tabId = sender && sender.tab ? sender.tab.id : -1;

  if (message.action === "getCosmetics") {
    chrome.storage.local.get({ enabled: true }, async (data) => {
      if (!data.enabled) {
        sendResponse({ css: "" });
        return;
      }
      const [css, scriptlets] = await Promise.all([
        getCosmeticCss(message.url),
        getScriptlets(message.url)
      ]);

      // Execute scriptlets in page MAIN world via chrome.scripting (bypasses page CSP)
      if (tabId >= 0 && scriptlets && scriptlets.length > 0 && chrome.scripting) {
        try {
          await chrome.scripting.executeScript({
            target: { tabId },
            world: "MAIN",
            func: runGenericScriptlets,
            args: [scriptlets]
          });
        } catch (_) {}
      }

      sendResponse({ css });
    });
    return true; // Keep message channel open for async response
  }

  if (message.action === "reportCosmetics") {
    if (tabId >= 0 && message.count > 0) {
      incrementBadge(tabId, message.count);
    }
    return false;
  }

  if (message.action === "toggle") {
    setRulesetEnabled(message.enabled);

    if (!message.enabled) {
      // Clear all badges when protection is toggled OFF
      tabCounts.clear();
      chrome.tabs.query({}, (tabs) => {
        for (const tab of tabs) {
          if (tab.id) setBadge(tab.id, 0);
        }
      });
    }

    // Notify all open tabs to update cosmetic styles
    chrome.tabs.query({}, (tabs) => {
      for (const tab of tabs) {
        if (tab.id) {
          chrome.tabs.sendMessage(tab.id, {
            action: "stateChanged",
            enabled: message.enabled
          }).catch(() => {});
        }
      }
    });
    sendResponse({ success: true });
    return false;
  }
});

// Initialize on install or startup
chrome.runtime.onInstalled.addListener(() => {
  chrome.storage.local.get({ enabled: true }, (data) => {
    setRulesetEnabled(data.enabled);
  });
  // Schedule anonymous daily rule update check
  chrome.alarms.create("rustad_daily_rules_update", { periodInMinutes: 1440 });
});

// Periodic anonymous rule updater (zero telemetry, zero cookies, zero user data)
const REMOTE_RULES_URL = "https://raw.githubusercontent.com/amaranthic/rustad/main/rustad/rules.json";

async function updateRulesAnonymously() {
  try {
    const response = await fetch(REMOTE_RULES_URL, {
      cache: "no-cache",
      credentials: "omit"
    });
    if (!response.ok) return;
    const newRules = await response.json();
    if (!Array.isArray(newRules) || newRules.length === 0) return;

    const existing = await chrome.declarativeNetRequest.getDynamicRules();
    const removeRuleIds = existing.map(r => r.id);
    const safeNewRules = newRules.slice(0, 30000).map((r, idx) => ({
      ...r,
      id: idx + 100000
    }));

    await chrome.declarativeNetRequest.updateDynamicRules({
      removeRuleIds,
      addRules: safeNewRules
    });
  } catch (_) {
    // Silent fail if offline or unreachable
  }
}

chrome.alarms.onAlarm.addListener((alarm) => {
  if (alarm && alarm.name === "rustad_daily_rules_update") {
    chrome.storage.local.get({ enabled: true }, (data) => {
      if (data.enabled) {
        updateRulesAnonymously();
      }
    });
  }
});
