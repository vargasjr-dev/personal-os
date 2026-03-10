# PersonalOS Quick Start Guide

Get your assistant-native OS booting in QEMU in under 10 minutes.

---

## Step 1: Install Rust (5 minutes)

```bash
# Install rustup
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Follow prompts, then:
source ~/.cargo/env

# Install nightly toolchain
rustup install nightly
rustup default nightly

# Add components for bare metal
rustup component add rust-src --toolchain nightly
rustup component add llvm-tools-preview --toolchain nightly
```

**Verify:**
```bash
rustc --version
# Should show: rustc 1.x.x-nightly
```

---

## Step 2: Install QEMU (2 minutes)

**Ubuntu/Debian:**
```bash
sudo apt update
sudo apt install qemu-system-x86
```

**Arch Linux:**
```bash
sudo pacman -S qemu
```

**macOS:**
```bash
brew install qemu
```

**Windows:**
Download from [qemu.org](https://www.qemu.org/download/) or use WSL2 with Ubuntu.

**Verify:**
```bash
qemu-system-x86_64 --version
```

---

## Step 3: Install bootimage (1 minute)

```bash
cargo install bootimage
```

This tool creates bootable disk images from your Rust kernel.

---

## Step 4: Clone & Build (2 minutes)

```bash
# Clone the repo
git clone https://github.com/vargasjr-dev/personal-os.git
cd personal-os

# Build and run in QEMU
cargo run
```

**What you'll see:**
A QEMU window opens with:
```
╔═══════════════════════════════════════════════════════════╗
║                                                           ║
║              PersonalOS - Assistant-Native OS             ║
║                                                           ║
║  "The future of computing starts here." ⚔️                ║
║                                                           ║
╚═══════════════════════════════════════════════════════════╝

Kernel booted successfully!
Architecture: x86_64
LLM Backend: Ready to connect

Testing LLM abstraction layer...

[INFO] LLM interface initialized
[INFO] Backend: Anthropic API (cloud) OR Local Llama
[INFO] Swap backends via environment/config

System ready. Halting...
```

**Close the window** or press Ctrl+C to exit.

---

## Step 5: Start Hacking! 🚀

Your OS is booting! Now you can:

### Edit the kernel

```bash
# Open the main kernel file
vim src/main.rs

# Make changes, then rebuild
cargo run
```

### Explore the LLM abstraction

```bash
# Check out the LLM backends
cat src/llm/mod.rs         # Core trait definition
cat src/llm/anthropic.rs   # Cloud API backend
cat src/llm/local.rs       # Local GPU backend
```

### Add new features

Ideas to implement:
- Keyboard input handler
- Simple shell/REPL
- Network stack (for Anthropic API)
- Filesystem driver (for local models)
- Memory allocator improvements

---

## Troubleshooting

**"error: linking with `rust-lld` failed"**
- Run: `rustup component add llvm-tools-preview --toolchain nightly`

**"QEMU not found"**
- Make sure QEMU is installed: `qemu-system-x86_64 --version`
- Add to PATH if needed

**"could not compile `bootloader`"**
- Ensure you're using nightly: `rustup default nightly`
- Update rust-src: `rustup component add rust-src --toolchain nightly`

**Build is slow**
- First build takes 2-5 minutes (compiling bootloader)
- Subsequent builds are much faster (seconds)

---

## What's Next?

1. **Read the code** - Start in `src/main.rs`, explore from there
2. **Study the README** - Understand the architecture
3. **Check SPECS.md** - Plan your hardware build
4. **Join the journey** - Build the future of computing! ⚔️

---

**Need help?** Open an issue on GitHub or check the [README](README.md) for more details.

**Ready to contribute?** PRs welcome! Let's build this together.
