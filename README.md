# PersonalOS - Assistant-Native Operating System

**Vision:** Build an operating system where AI assistants are first-class citizens, not applications. Challenge Windows and Mac with an AI-first computing paradigm.

**Status:** Early development - bootable kernel with LLM abstraction layer

---

## 🚀 Quick Start

**Get booting in under 5 minutes:**

```bash
# Clone the repo
git clone https://github.com/vargasjr-dev/personal-os.git
cd personal-os

# Run automated setup
./setup.sh
source ~/.cargo/env

# Build and run in QEMU
cargo run
```

The `setup.sh` script automatically installs Rust (nightly), QEMU, and all dependencies.

**Supported systems:** Ubuntu, Debian, Arch, Fedora, macOS

See [QUICKSTART.md](QUICKSTART.md) for detailed instructions and troubleshooting.

---

## 🧠 LLM Abstraction Layer

PersonalOS includes a swappable LLM backend system. You can easily switch between:

1. **Anthropic API** (cloud) - Claude Opus 4.6
2. **Local Llama** (on-device) - Llama 3.1 70B on RTX 4090

### Architecture

```
┌─────────────────────────────────────┐
│         Kernel / Applications       │
│      (Calls LlmBackend trait)       │
└──────────────┬──────────────────────┘
               │
      ┌────────▼────────┐
      │   LlmManager    │  ← Manages active backend
      └────────┬────────┘
               │
       ┌───────┴───────┐
       │               │
┌──────▼──────┐ ┌─────▼──────┐
│  Anthropic  │ │   Local    │
│   Backend   │ │   Llama    │
│  (API call) │ │ (GPU/CPU)  │
└─────────────┘ └────────────┘
```

### Code Structure

```
src/
├── main.rs              # Kernel entry point
├── vga_buffer.rs        # VGA text mode driver
└── llm/
    ├── mod.rs           # LlmBackend trait + manager
    ├── anthropic.rs     # Anthropic API implementation
    └── local.rs         # Local Llama implementation
```

### Usage Example

```rust
use llm::{LlmManager, LlmBackend};

// Use Anthropic API (cloud)
let manager = LlmManager::new(false);
let response = manager.query("Explain quantum computing")?;

// Switch to local Llama (on-device)
let mut manager = LlmManager::new(true);
let response = manager.query("Explain quantum computing")?;
```

The calling code doesn't change - just swap the backend!

---

## 🎯 Current Status

### ✅ Implemented
- Bare metal Rust kernel
- Bootloader integration
- VGA text mode driver (println! macro)
- LLM abstraction layer (trait-based)
- Anthropic backend stub
- Local Llama backend stub
- QEMU support

### 🚧 In Progress
- Network stack (for Anthropic API calls)
- Filesystem driver (for local model loading)
- CUDA driver integration (for GPU acceleration)

### 📋 Roadmap
- [ ] TCP/IP + TLS stack
- [ ] Anthropic API client (HTTPS + JSON)
- [ ] GPU drivers (NVIDIA CUDA)
- [ ] Model loader (GGUF format)
- [ ] Inference engine (llama.cpp port)
- [ ] Keyboard/mouse input
- [ ] Basic shell
- [ ] Assistant-native UI paradigm

---

## 🔧 Development Workflow

### Testing in QEMU

The default `cargo run` boots the OS in QEMU. This is how you'll develop until you have the physical hardware.

**What works in QEMU:**
- ✅ All kernel code
- ✅ Memory management
- ✅ Interrupts and syscalls
- ✅ Network stack testing
- ✅ Basic UI/shell

**What requires real hardware:**
- GPU-accelerated LLM inference
- Full-speed performance testing
- Hardware driver development

### Building for Real Hardware

Once you have the RTX 4090 workstation:

```bash
# Build bootable ISO
cargo bootimage

# Write to USB stick (replace /dev/sdX with your USB device)
sudo dd if=target/x86_64-personal_os/release/bootimage-personal-os.bin of=/dev/sdX bs=4M && sync

# Boot from USB on real hardware
```

---

## 💾 Hardware Target

**Development machine:** $3,720 workstation
- AMD Ryzen 9 7950X (16-core)
- NVIDIA RTX 4090 24GB
- 128GB DDR5 RAM
- 2TB NVMe SSD

**LLM Performance:**
- Llama 3.1 70B: 40+ tokens/sec
- Llama 3.1 13B: 120+ tokens/sec
- Multiple small models simultaneously

See [SPECS.md](SPECS.md) for complete hardware details.

---

## 🔀 Swapping LLM Backends

The system is designed to seamlessly switch between cloud and local inference:

**Use Anthropic API when:**
- No GPU available (laptop, VM, etc.)
- Internet connection reliable
- Want latest models without local storage

**Use Local Llama when:**
- Have GPU hardware (RTX 4090)
- Need offline capability
- Privacy-sensitive workloads
- Want maximum speed (40+ tok/s)

**Switching at runtime:**
```rust
let mut manager = LlmManager::new(false); // Start with Anthropic
manager.switch_backend(true);             // Switch to local Llama
```

---

## 🛠️ Technical Details

### No Standard Library

This OS is built with `#![no_std]` - it doesn't use the Rust standard library. Everything is implemented from scratch:

- Memory allocation
- String handling
- I/O operations
- Networking
- Filesystem

### Custom Target

The OS compiles to a custom target (`x86_64-personal_os.json`) for bare metal x86_64. No operating system underneath - this **is** the operating system.

### Bootloader

Uses the `bootloader` crate which:
1. Loads kernel into memory
2. Switches to long mode (64-bit)
3. Maps memory properly
4. Jumps to `_start()`

---

## 📚 Resources

**Learning OS development:**
- [Writing an OS in Rust](https://os.phil-opp.com/) - Excellent tutorial series
- [OSDev Wiki](https://wiki.osdev.org/) - Comprehensive OS dev reference
- [Redox OS](https://www.redox-os.org/) - Mature Rust OS to study

**Rust bare metal:**
- [Rust Embedded Book](https://rust-embedded.github.io/book/)
- [x86_64 crate docs](https://docs.rs/x86_64/)

**LLM integration:**
- [llama.cpp](https://github.com/ggerganov/llama.cpp) - Fast LLM inference
- [Anthropic API docs](https://docs.anthropic.com/) - Claude API reference

---

## 🎭 Philosophy

**Assistant-native means:**
- Natural language is a primary interface (not just CLI/GUI)
- Context flows between everything (no app silos)
- System anticipates needs (proactive, not reactive)
- Permission model for assistants (not just files)
- Memory & continuity built-in (the OS remembers)

**Not a chatbot in an app. The OS itself is assistant-native.**

---

## 🤝 Contributing

This is an ambitious project. Contributions welcome:

- Kernel features (interrupts, syscalls, etc.)
- Driver development (GPU, network, storage)
- LLM integration improvements
- UI/UX design for assistant-native paradigm
- Documentation and tutorials

---

## ⚔️ The Future

**"The future of computing starts here."**

Windows and Mac weren't built for an AI-first world. PersonalOS is. When assistants are OS primitives, everything changes:

- **No more app switching** - Just ask your OS
- **Context everywhere** - The system knows what you're doing
- **Proactive assistance** - The OS anticipates needs
- **Privacy by default** - All intelligence on-device

This is the conviction. Now we build it.

---

*Built with Rust 🦀 | Powered by AI 🤖 | Driven by Conviction ⚔️*
