#!/bin/bash
# xdotter installation script
# Usage: curl -sSL https://raw.githubusercontent.com/cncsmonster/xdotter/main/install.sh | bash

set -e

# Configuration
REPO="cncsmonster/xdotter"
SCRIPT_NAME="xd"
INSTALL_DIR="$HOME/.local/bin"
# Download from GitHub Releases (latest version)
SCRIPT_URL="https://github.com/$REPO/releases/latest/download/xd.pyz"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

info() {
    echo -e "${GREEN}✓${NC} $1"
}

warn() {
    echo -e "${YELLOW}!${NC} $1"
}

error() {
    echo -e "${RED}✗${NC} $1"
}

# Check for Python 3.8+
check_python() {
    if command -v python3 &> /dev/null; then
        PYTHON_CMD="python3"
    elif command -v python &> /dev/null; then
        PYTHON_CMD="python"
    else
        error "Python 3 is required but not found"
        exit 1
    fi

    # Check version (need 3.8+)
    PYTHON_VERSION=$($PYTHON_CMD -c 'import sys; print(f"{sys.version_info.major}.{sys.version_info.minor}")')
    REQUIRED_VERSION="3.8"

    if [ "$(printf '%s\n' "$REQUIRED_VERSION" "$PYTHON_VERSION" | sort -V | head -n1)" != "$REQUIRED_VERSION" ]; then
        error "Python 3.8+ is required, found Python $PYTHON_VERSION"
        exit 1
    fi

    info "Found Python $PYTHON_VERSION"
}

# Create installation directory
setup_install_dir() {
    if [ ! -d "$INSTALL_DIR" ]; then
        info "Creating installation directory: $INSTALL_DIR"
        mkdir -p "$INSTALL_DIR"
    fi

    # Check if INSTALL_DIR is in PATH
    if [[ ":$PATH:" != *":$INSTALL_DIR:"* ]]; then
        warn "$INSTALL_DIR is not in your PATH"
        echo ""
        echo "Add this to your shell configuration file:"
        echo "  export PATH=\"$INSTALL_DIR:\$PATH\""
        echo ""
        echo "For bash: add to ~/.bashrc"
        echo "For zsh: add to ~/.zshrc"
        echo ""
    fi
}

# Download the single-file executable
download_script() {
    info "Downloading xdotter..."

    if command -v curl &> /dev/null; then
        curl -sSL "$SCRIPT_URL" -o "$INSTALL_DIR/$SCRIPT_NAME"
    elif command -v wget &> /dev/null; then
        wget -q "$SCRIPT_URL" -O "$INSTALL_DIR/$SCRIPT_NAME"
    else
        error "Neither curl nor wget found. Please install one of them."
        exit 1
    fi

    chmod +x "$INSTALL_DIR/$SCRIPT_NAME"
    info "Installed to: $INSTALL_DIR/$SCRIPT_NAME"
}

# Verify installation
verify_installation() {
    if [ -f "$INSTALL_DIR/$SCRIPT_NAME" ] && [ -x "$INSTALL_DIR/$SCRIPT_NAME" ]; then
        info "Installation successful!"
        echo ""
        echo "You can now use xdotter by running:"
        echo "  $SCRIPT_NAME --help"
        echo ""
    else
        error "Installation failed"
        exit 1
    fi
}

# Main installation
main() {
    echo "╔══════════════════════════════════════╗"
    echo "║     xdotter Installation Script      ║"
    echo "╚══════════════════════════════════════╝"
    echo ""

    check_python
    setup_install_dir
    download_script
    verify_installation

    echo "Quick start:"
    echo "  $SCRIPT_NAME new          # Create a new config"
    echo "  $SCRIPT_NAME deploy       # Deploy dotfiles"
    echo "  $SCRIPT_NAME --help       # Show help"
}

# Run installation
main "$@"