// ==============================================================================
// RUSTAD WORKER (worker.js)
// Service worker managing declarative network rules and WASM cosmetics engine
// ==============================================================================

let wasmInstance = null;
let memoryBuffer = null;

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

  // Refresh buffer in case of memory growth
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

  if (message.action === "toggle") {
    setRulesetEnabled(message.enabled);

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
