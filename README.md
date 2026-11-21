<p align="center">
  <img src="rustad/icons/128.png" alt="Rustad Logo" width="100" height="100" style="border-radius: 22%;">
</p>

<h1 align="center">Rustad</h1>

<p align="center">
  <b>An ad is an ad.</b><br>
  The zero-dependency, ultra-fast WebAssembly ad-blocking engine written in pure Rust.
</p>

<p align="center">
  <img src="https://img.shields.io/badge/Rust-100%25_Pure_Stdlib-orange?style=flat-square&logo=rust" alt="Pure Rust">
  <img src="https://img.shields.io/badge/Dependencies-0_Crates.io-brightgreen?style=flat-square" alt="Zero Dependencies">
  <img src="https://img.shields.io/badge/WASM_Size-~53_KB-blue?style=flat-square&logo=webassembly" alt="WASM Size">
  <img src="https://img.shields.io/badge/Manifest-v3_Compliant-purple?style=flat-square&logo=googlechrome" alt="Manifest V3">
  <img src="https://img.shields.io/badge/License-MIT_OR_Apache--2.0-yellow?style=flat-square" alt="License">
</p>

---

## ⚡ Why Rustad?

Modern web ad blockers have become bloated with 15 years of technical debt: 300,000+ outdated rules, contradictory whitelist exceptions that quietly allow Outbrain and Taboola onto media portals, megabytes of bloated JavaScript bundles, and heavy memory footprints.

**Rustad was built on a single, uncompromising rule: *An ad is an ad.***

Engineered from scratch in pure standard library Rust, Rustad compiles to a microscopic **~53 KB WebAssembly binary** that runs in browser service workers with zero runtime heap allocations, instantaneous $O(\log N)$ lookups, and uncompromising blocking power.

---

## ✨ Key Architectural Highlights

* **Zero Crates.io Dependencies**: Built exclusively with Rust standard primitives. No external dependencies, no unsafe third-party crates, no dependency supply-chain risks.
* **Microscopic Footprint**: The entire release WebAssembly engine (`rustad.wasm`) weighs just **~53 KB**. Starts up in **0 ms** with virtually zero RAM footprint.
* **Compile-Time Assembly (`build.rs`)**: 100% of rule parsing, normalization, canonical sorting, and buffer size calculations happen at compile time. The runtime binary contains only pre-indexed static slices.
* **Zero-Allocation Memory Protocol**: Host environments exchange URLs and retrieve generated CSS/JSON using a static C-ABI shared memory protocol (`rustad_input_ptr` / `rustad_output_ptr`).
* **Clean Manifest V3 Architecture**: Native Chrome Manifest V3 extension featuring declarative network request protection, service workers, and surgical DOM shielding.

---

## 🛡️ Four-Layer Defense Matrix

Rustad defends your browsing experience through four specialized, coordinated layers:

```
┌────────────────────────────────────────────────────────────────────────┐
│                        RUSTAD 4-LAYER DEFENSE                          │
└───────────────────────────────────┬────────────────────────────────────┘
                                    │
    ┌───────────────────────────────┼───────────────────────────────┐
    │                               │                               │
    ▼                               ▼                               ▼
1. NETWORK LAYER            2. COSMETIC LAYER               3. SCRIPTLET LAYER
(Chrome MV3 DNR)            (Rust WASM Engine)             (Main World Injection)
Blocks ad servers,          Zero-pixel CSS collapse:        Defuses video ad streams
trackers & telemetry        display: none !important;       (YouTube prerolls) and
before packets wire.        opacity: 0, 0 margin/padding.   anti-adblock traps.
    │                               │                               │
    └───────────────────────────────┼───────────────────────────────┘
                                    │
                                    ▼
                        4. ANONYMOUS LIVE UPDATER
                        (Automated Chrome Alarm)
                        Pulls clean rules daily via
                        updateDynamicRules with zero
                        cookies and zero telemetry.
```

### 1. Network Layer (DeclarativeNetRequest)
Pre-filtered, 100% ASCII-compliant ruleset operating at the browser engine level. Intercepts and blocks tracking beacons, telemetry endpoints (Bugsnag, Sentry, Cloudflare Insights), and ad servers before requests leave the computer.

### 2. Cosmetic Layer (Rust WASM CSS Engine)
When ad containers are removed from the network, empty boxes or layout gaps often remain. Rustad's WebAssembly engine queries domain and global selectors in microseconds and injects an extreme CSS layout collapse:
```css
{
  display: none !important;
  visibility: hidden !important;
  opacity: 0 !important;
  pointer-events: none !important;
  height: 0 !important;
  min-height: 0 !important;
  max-height: 0 !important;
  width: 0 !important;
  min-width: 0 !important;
  max-width: 0 !important;
  margin: 0 !important;
  padding: 0 !important;
  border: none !important;
  overflow: hidden !important;
  line-height: 0 !important;
  font-size: 0 !important;
}
```

### 3. Scriptlet Layer (Main World Runtime Injection)
Server-side ad insertion (like YouTube video prerolls and unskippable ads) and anti-adblock traps cannot be stopped by CSS alone. Rustad runs surgical micro-primitives inside the page's `MAIN` execution context via `chrome.scripting` to neutralize ad placement metadata (`ytInitialPlayerResponse.adPlacements`) before players initialize.

### 4. Anonymous Live Rule Updater
A background alarm periodically checks for updated rulesets, seamlessly applying fresh rules using `chrome.declarativeNetRequest.updateDynamicRules`. **100% anonymous**: zero telemetry, zero cookies, zero user analytics.

---

## 🚀 Installation

### Option A: Load Unpacked .zip (Recommended for Standard Google Chrome)
> [!NOTE]
> Official Google Chrome strictly blocks off-store `.crx` drag-and-drop installations with `CRX_REQUIRED_PROOF_MISSING` (requiring Chrome Web Store cryptographic proofs). For Chrome, use the `.zip` package via **Load unpacked**:
1. Download `rustad_vX.Y.Z.zip` from the [Latest Release](https://github.com/amaranthic/rustad/releases).
2. Unzip the archive into a folder.
3. Navigate to `chrome://extensions` and turn on **Developer mode** (toggle in the top-right corner).
4. Click **Load unpacked** in the top-left and select the unzipped directory.

### Option B: Packaged .crx (Brave, Edge, Chromium)
1. Download `rustad_vX.Y.Z.crx` from the [Latest Release](https://github.com/amaranthic/rustad/releases).
2. Open `chrome://extensions` and ensure **Developer mode** is enabled.
3. Drag and drop the `.crx` file directly into the browser window to install.

---

## 🛠️ Building from Source

### Prerequisites
* Rust toolchain (1.75+ recommended):
  ```bash
  rustup target add wasm32-unknown-unknown
  ```

### Build Instructions
```bash
# 1. Clone the repository
git clone https://github.com/amaranthic/rustad.git
cd rustad

# 2. Run unit tests and verification
cargo test

# 3. Check linter rules
cargo clippy --all-targets

# 4. Compile the WebAssembly release binary
cargo build --target wasm32-unknown-unknown --release

# 5. Copy the generated WASM binary into the extension folder
cp target/wasm32-unknown-unknown/release/rustad.wasm rustad/rustad.wasm
```

Now load the `rustad/` folder into your browser as an unpacked extension.

---

## 📂 Project Architecture

```
rustad/
├── Cargo.toml                 ← Rust package configuration & strict linter flags
├── build.rs                   ← Compile-time rule processor & code generator
│
├── rules/                     ← Human-curated, surgical rule source files
│   ├── domains.txt            ← Extreme ad & tracker domain blocklist
│   ├── patterns.txt           ← High-impact URL path & wildcard rules
│   ├── exceptions.txt         ← Strict unbreak whitelist entries
│   ├── cosmetics.txt          ← Element-hiding CSS selectors
│   └── scriptlets.txt         ← Video ad defusers & anti-adblock stubs
│
├── src/                       ← Pure Rust engine (0 crates.io dependencies)
│   ├── lib.rs                 ← C-ABI WASM exports & static memory buffers
│   ├── engine.rs              ← URL classification & cosmetic coordination
│   ├── domain.rs              ← Hostname extraction & O(log N) binary search
│   ├── pattern.rs             ← Zero-allocation wildcard & separator glob matcher
│   ├── cosmetic.rs            ← CSS selector resolution & subdomain walking
│   ├── scriptlet.rs           ← Scriptlet lookup & compact JSON serializer
│   └── generated/
│       └── rules.rs           ← AUTO-GENERATED compile-time static slices
│
├── rustad/                    ← Packaged Chrome Manifest V3 extension
│   ├── manifest.json          ← Extension manifest & permission declarations
│   ├── rules.json             ← Compiled declarativeNetRequest static ruleset
│   ├── rustad.wasm            ← Compiled ~53 KB WebAssembly binary
│   ├── worker.js              ← Background service worker & badge manager
│   ├── shield.js              ← Generic 0ms content script & CSS injector
│   ├── icons/                 ← App icons (16px, 48px, 128px)
│   └── popup/                 ← Minimal dark-mode control interface
│
└── .github/
    └── workflows/
        └── release.yml        ← Automated daily release & asset packaging workflow
```

---

## 📄 License

Dual-licensed under either:
* **MIT License** ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)
* **Apache License, Version 2.0** ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)

at your option.
