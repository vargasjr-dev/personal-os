#!/bin/bash
set -e

# PersonalOS Setup Script
# Automates installation of Rust toolchain, QEMU, and dependencies

echo "╔═══════════════════════════════════════════════════════════╗"
echo "║                                                           ║"
echo "║           PersonalOS Development Environment Setup        ║"
echo "║                                                           ║"
echo "╚═══════════════════════════════════════════════════════════╝"
echo ""

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

info() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

error() {
    echo -e "${RED}[ERROR]${NC} $1"
    exit 1
}

# Check if running on supported OS
detect_os() {
    if [[ "$OSTYPE" == "linux-gnu"* ]]; then
        if [ -f /etc/os-release ]; then
            . /etc/os-release
            OS=$ID
        else
            error "Cannot detect Linux distribution"
        fi
    elif [[ "$OSTYPE" == "darwin"* ]]; then
        OS="macos"
    else
        error "Unsupported OS: $OSTYPE"
    fi
    info "Detected OS: $OS"
}

# Install Rust toolchain
install_rust() {
    if command -v rustc &> /dev/null; then
        info "Rust already installed: $(rustc --version)"
    else
        info "Installing Rust toolchain..."
        curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
        source "$HOME/.cargo/env"
        info "Rust installed successfully"
    fi
}

# Configure Rust nightly
configure_rust() {
    info "Configuring Rust nightly toolchain..."
    
    # Install nightly
    rustup install nightly
    rustup default nightly
    
    # Add required components
    rustup component add rust-src --toolchain nightly
    rustup component add llvm-tools-preview --toolchain nightly
    
    info "Rust nightly configured: $(rustc --version)"
}

# Install QEMU
install_qemu() {
    if command -v qemu-system-x86_64 &> /dev/null; then
        info "QEMU already installed: $(qemu-system-x86_64 --version | head -n1)"
        return
    fi
    
    info "Installing QEMU..."
    
    case $OS in
        ubuntu|debian|pop)
            sudo apt update
            sudo apt install -y qemu-system-x86
            ;;
        arch|manjaro)
            sudo pacman -S --noconfirm qemu
            ;;
        fedora|rhel|centos)
            sudo dnf install -y qemu-system-x86
            ;;
        macos)
            if ! command -v brew &> /dev/null; then
                error "Homebrew not found. Install from https://brew.sh"
            fi
            brew install qemu
            ;;
        *)
            warn "Unknown OS: $OS. Please install QEMU manually:"
            echo "  Ubuntu/Debian: sudo apt install qemu-system-x86"
            echo "  Arch: sudo pacman -S qemu"
            echo "  Fedora: sudo dnf install qemu-system-x86"
            echo "  macOS: brew install qemu"
            exit 1
            ;;
    esac
    
    info "QEMU installed successfully"
}

# Install bootimage tool
install_bootimage() {
    if command -v bootimage &> /dev/null; then
        info "bootimage already installed"
        return
    fi
    
    info "Installing bootimage tool..."
    cargo install bootimage
    info "bootimage installed successfully"
}

# Verify installation
verify_setup() {
    info "Verifying installation..."
    
    # Check Rust
    if ! command -v rustc &> /dev/null; then
        error "Rust installation failed"
    fi
    
    # Check nightly
    if ! rustc --version | grep -q "nightly"; then
        error "Rust nightly not set as default"
    fi
    
    # Check QEMU
    if ! command -v qemu-system-x86_64 &> /dev/null; then
        error "QEMU installation failed"
    fi
    
    # Check bootimage
    if ! command -v bootimage &> /dev/null; then
        error "bootimage installation failed"
    fi
    
    info "✅ All components verified!"
}

# Main installation flow
main() {
    detect_os
    echo ""
    
    info "Step 1/4: Installing Rust toolchain..."
    install_rust
    echo ""
    
    info "Step 2/4: Configuring Rust nightly..."
    configure_rust
    echo ""
    
    info "Step 3/4: Installing QEMU..."
    install_qemu
    echo ""
    
    info "Step 4/4: Installing bootimage..."
    install_bootimage
    echo ""
    
    verify_setup
    echo ""
    
    echo "╔═══════════════════════════════════════════════════════════╗"
    echo "║                                                           ║"
    echo "║                  Setup Complete! ✅                        ║"
    echo "║                                                           ║"
    echo "╚═══════════════════════════════════════════════════════════╝"
    echo ""
    echo "Next steps:"
    echo "  1. Source Rust environment:  source ~/.cargo/env"
    echo "  2. Build and run the OS:     cargo run"
    echo "  3. Start hacking! ⚔️"
    echo ""
}

main
