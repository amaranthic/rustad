# Rustad

High-performance, zero-dependency browser-agnostic adblocking engine in pure Rust and WebAssembly.

## Features

- **Zero Crates.io Dependencies**: Built exclusively on standard Rust language primitives.
- **Ultra-Fast $O(\log N)$ Matching**: Compile-time sorted static arrays with binary search.
- **Zero Runtime Allocations**: All matching runs against byte slices (`&[u8]` and `&str`).
- **Tiny WebAssembly Footprint**: ~38 KB `.wasm` output.
- **Strict Quality**: Thorough documentation, zero warnings, and clean C-ABI exports.
