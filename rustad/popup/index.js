const toggle = document.getElementById("toggle");

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
