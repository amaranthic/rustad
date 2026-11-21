const toggle = document.getElementById("toggle");
const versionEl = document.getElementById("version");

// Display dynamic version from manifest
const manifest = chrome.runtime.getManifest();
if (versionEl && manifest && manifest.version) {
  versionEl.textContent = `v${manifest.version}`;
}

// Read current enabled state
chrome.storage.local.get({ enabled: true }, (data) => {
  toggle.checked = data.enabled;
});

// Broadcast toggle changes
toggle.addEventListener("change", () => {
  const enabled = toggle.checked;
  chrome.storage.local.set({ enabled }, () => {
    chrome.runtime.sendMessage({ action: "toggle", enabled });
  });
});
