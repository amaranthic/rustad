// ==============================================================================
// RUSTAD PAGE SHIELD (shield.js)
// Injects cosmetic hiding styles at document_start; updates on toggle
// ==============================================================================

(function () {
  const STYLE_ID = "rustad-shield";

  function applyStyles(css) {
    if (!css) {
      removeStyles();
      return;
    }

    let el = document.getElementById(STYLE_ID);
    if (!el) {
      el = document.createElement("style");
      el.id = STYLE_ID;
      (document.head || document.documentElement).appendChild(el);
    }
    el.textContent = css;
  }

  function removeStyles() {
    const el = document.getElementById(STYLE_ID);
    if (el) {
      el.remove();
    }
  }

  function refresh() {
    chrome.runtime.sendMessage(
      { action: "getCosmetics", url: window.location.href },
      (response) => {
        if (chrome.runtime.lastError || !response) {
          return;
        }
        applyStyles(response.css);
      }
    );
  }

  // Initial shield on page load
  refresh();

  // Listen for real-time toggle changes from the worker
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
