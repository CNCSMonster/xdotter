#!/bin/bash
# xdotter installation script
# Usage: curl -sSL https://raw.githubusercontent.com/cncsmonster/xdotter/main/install.sh | bash

set -e

# Configuration
REPO="cncsmonster/xdotter"
SCRIPT_NAME="xd"
INSTALL_DIR="$HOME/.local/bin"
SCRIPT_URL="https://raw.githubusercontent.com/$REPO/main/xd.py"

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

# Check for Python 3
check_python() {
    if command -v python3 &> /dev/null; then
        PYTHON_CMD="python3"
    elif command -v python &> /dev/null; then
        PYTHON_VERSION=$(python --version 2>&1 | grep -oP '\d\.\d+' | head -1)
        PYTHON_MAJOR=$(echo $PYTHON_VERSION | cut -d. -f1)
        if [ "$PYTHON_MAJOR" -ge 3 ]; then
            PYTHON_CMD="python"
        else
            error "Python 3 is required but not found"
            exit 1
        fi
    else
        error "Python 3 is required but not found"
        exit 1
    fi
    
    info "Found Python: $PYTHON_CMD"
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

# Download the script
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

# Create wrapper script for convenience
create_wrapper() {
    cat > "$INSTALL_DIR/$SCRIPT_NAME" << 'WRAPPER'
#!/bin/bash
# xdotter wrapper script
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
XD_SCRIPT="$SCRIPT_DIR/xd.py"

if [ -f "$XD_SCRIPT" ]; then
    exec python3 "$XD_SCRIPT" "$@"
else
    echo "Error: xd.py not found at $XD_SCRIPT"
    exit 1
fi
WRAPPER

    chmod +x "$INSTALL_DIR/$SCRIPT_NAME"
}

# Verify installation
verify_installation() {
    if [ -f "$INSTALL_DIR/$SCRIPT_NAME" ] && [ -x "$INSTALL_DIR/$SCRIPT_NAME" ]; then
        info "Installation successful!"
        echo ""
        echo "You can now use xdotter by running:"
        echo "  $SCRIPT_NAME --help"
        echo ""
        echo "Or add to PATH and use directly:"
        echo "  export PATH=\"$INSTALL_DIR:\$PATH\""
        echo ""
    else
        error "Installation failed"
        exit 1
    fi
}

# Alternative: Install to /usr/local/bin (requires sudo)
install_system_wide() {
    if [ "$EUID" -eq 0 ] || command -v sudo &> /dev/null; then
        info "Installing system-wide to /usr/local/bin..."
        local install_cmd="cp"
        local chmod_cmd="chmod"
        
        if [ "$EUID" -ne 0 ]; then
            install_cmd="sudo cp"
            chmod_cmd="sudo chmod"
        fi
        
        curl -sSL "$SCRIPT_URL" -o /usr/local/bin/$SCRIPT_NAME
        $chmod_cmd +x /usr/local/bin/$SCRIPT_NAME
        info "Installed to: /usr/local/bin/$SCRIPT_NAME"
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
    echo "  $SCRIPT_NAME --help       # Show help"
}

# Run installation
main "$@"
