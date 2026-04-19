# PersonalOS — Assistant-Native Operating System

**A bare-metal x86_64 operating system written in Rust where AI is a first-class citizen at the kernel level.**

Built by [VargasJR](https://vargasjr.dev) ⚔️

## Status

Early development — bootable kernel with VGA output, keyboard input, and an LLM abstraction layer.

## What's Here

```
src/
  main.rs         — Kernel entry point, VGA boot screen
  vga_buffer.rs   — Text-mode VGA driver (80×25, colors, scrolling)
  llm/            — LLM abstraction layer (provider traits, prompt/response types)
```

## Build & Run

Requires Rust nightly, `cargo-bootimage`, and QEMU:

```bash
# Install Rust nightly with required components
rustup toolchain install nightly
rustup component add rust-src llvm-tools-preview --toolchain nightly

# Install bootimage
cargo install bootimage

# Build and run in QEMU
cargo run
```

## Test

```bash
# Requires QEMU for integration tests
cargo test
```

## Architecture

- **Target:** `x86_64-personal_os` (custom target, no OS)
- **Bootloader:** `bootloader` crate v0.9 with physical memory mapping
- **No std:** `#![no_std]`, `#![no_main]`, custom panic handler
- **Test framework:** Custom test runner with QEMU exit device

## Hardware Specs

See [SPECS.md](SPECS.md) for the target development workstation build ($3,720 — RTX 4090, Ryzen 9 7950X, 64GB DDR5).

## License

MIT
