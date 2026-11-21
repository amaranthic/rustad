// ==============================================================================
// RUSTAD PAGE SHIELD (shield.js)
// 100% Generic, site-agnostic cosmetic CSS injector
// ==============================================================================

(function () {
  const STYLE_ID = "rustad-shield";
  const CACHE_KEY = "rustad_css_cache";

  function applyStyles(css) {
    if (!css) {
      removeStyles();
      return;
    }

    let el = document.getElementById(STYLE_ID);
    if (!el) {
      el = document.createElement("style");
      el.id = STYLE_ID;
      const parent = document.head || document.documentElement;
      if (parent) {
        parent.insertBefore(el, parent.firstChild);
      }
    }
    if (el.textContent !== css) {
      el.textContent = css;
    }
  }

  function removeStyles() {
    const el = document.getElementById(STYLE_ID);
    if (el) {
      el.remove();
    }
    try {
      sessionStorage.removeItem(CACHE_KEY);
    } catch (_) {}
  }

  // 1. Instant Synchronous Cache Injection (0ms delay at document_start)
  try {
    const cached = sessionStorage.getItem(CACHE_KEY);
    if (cached) {
      applyStyles(cached);
    }
  } catch (_) {}

  // 2. Fetch fresh cosmetic rules from worker via Rust WebAssembly
  function refresh() {
    chrome.runtime.sendMessage(
      { action: "getCosmetics", url: window.location.href },
      (response) => {
        if (chrome.runtime.lastError || !response) {
          return;
        }
        applyStyles(response.css);
        try {
          if (response.css) {
            sessionStorage.setItem(CACHE_KEY, response.css);
          } else {
            sessionStorage.removeItem(CACHE_KEY);
          }
        } catch (_) {}
      }
    );
  }

  refresh();

  // 3. Listen for real-time toggle changes from the popup / worker
  chrome.runtime.onMessage.addListener((msg) => {
    if (msg && msg.action === "stateChanged") {
      if (msg.enabled) {
        refresh();
      } else {
        removeStyles();
      }
    }
  });
})();
