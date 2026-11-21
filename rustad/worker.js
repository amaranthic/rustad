// ==============================================================================
// RUSTAD WORKER (worker.js)
// Service worker managing declarative network rules, WASM cosmetics, and in-memory badge
// ==============================================================================

let wasmInstance = null;
let memoryBuffer = null;

// In-memory badge counts per tab (strictly RAM only, resets on navigation)
const tabCounts = new Map();

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
    setBadge(tabId, 0);
  }
});

chrome.tabs.onRemoved.addListener((tabId) => {
  tabCounts.delete(tabId);
});

// Track network ad blocks in real time
if (chrome.declarativeNetRequest.onRuleMatchedDebug) {
  chrome.declarativeNetRequest.onRuleMatchedDebug.addListener((info) => {
    if (info && info.request && typeof info.request.tabId === "number") {
      incrementBadge(info.request.tabId, 1);
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
      const css = await getCosmeticCss(message.url);
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
});
