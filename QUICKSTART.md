# PersonalOS Quick Start Guide

Get your assistant-native OS booting in QEMU in under 5 minutes.

---

## Step 1: Clone the Repo

```bash
git clone https://github.com/vargasjr-dev/personal-os.git
cd personal-os
```

---

## Step 2: Run Setup Script

```bash
./setup.sh
```

This automatically installs:
- ✅ Rust toolchain (nightly)
- ✅ QEMU virtualization
- ✅ bootimage tool
- ✅ All required components

**Supported systems:** Ubuntu, Debian, Arch, Fedora, macOS

After setup completes:
```bash
source ~/.cargo/env
```

---

## Step 3: Build & Run

```bash
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

## Step 4: Start Hacking! 🚀

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

**Setup script failed?**
- Re-run with verbose output: `bash -x ./setup.sh`
- Check you have sudo access (needed for QEMU install)
- On macOS, ensure Homebrew is installed first

**"error: linking with `rust-lld` failed"**
- Run: `rustup component add llvm-tools-preview --toolchain nightly`

**"QEMU not found"**
- Make sure QEMU is installed: `qemu-system-x86_64 --version`
- Re-run setup: `./setup.sh`

**"could not compile `bootloader`"**
- Ensure you're using nightly: `rustup default nightly`
- Update rust-src: `rustup component add rust-src --toolchain nightly`

**Build is slow**
- First build takes 2-5 minutes (compiling bootloader)
- Subsequent builds are much faster (seconds)

**Manual installation (if setup.sh doesn't work)**
- Check the script contents: `cat setup.sh`
- Follow the commands manually for your OS

---

## What's Next?

1. **Read the code** - Start in `src/main.rs`, explore from there
2. **Study the README** - Understand the architecture
3. **Check SPECS.md** - Plan your hardware build
4. **Join the journey** - Build the future of computing! ⚔️

---

**Need help?** Open an issue on GitHub or check the [README](README.md) for more details.

**Ready to contribute?** PRs welcome! Let's build this together.
