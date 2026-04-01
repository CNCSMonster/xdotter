#!/bin/bash
# Build xd.pyz - single-file executable Python archive
# Usage: ./scripts/build-pyz.sh [--install]
#
# Options:
#   --install    Install to ~/.local/bin/xd after building

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"

cd "$PROJECT_DIR"

# Parse arguments
INSTALL=false
for arg in "$@"; do
    case $arg in
        --install)
            INSTALL=true
            shift
            ;;
        *)
            echo "Unknown option: $arg"
            echo "Usage: $0 [--install]"
            exit 1
            ;;
    esac
done

echo "Building xd.pyz..."

# Get build information
BUILD_TIME=$(date -u +"%Y-%m-%d %H:%M:%S UTC")
BUILD_COMMIT=$(git rev-parse --short HEAD 2>/dev/null || echo "unknown")

echo "  Build time: $BUILD_TIME"
echo "  Build commit: $BUILD_COMMIT"

python3 << EOF
import zipapp
import tempfile
import shutil
from pathlib import Path

with tempfile.TemporaryDirectory() as tmpdir:
    p = Path(tmpdir)

    # Copy main script
    shutil.copy('xd.py', p / 'xd.py')

    # Copy _vendor, excluding __pycache__ and .pyc files
    shutil.copytree(
        '_vendor',
        p / '_vendor',
        ignore=shutil.ignore_patterns('__pycache__', '*.pyc')
    )

    # Create build info module (embedded in pyz)
    build_info = f'''# Build information (auto-generated)
XD_BUILD_TIME = "$BUILD_TIME"
XD_BUILD_COMMIT = "$BUILD_COMMIT"
'''
    (p / 'build_info.py').write_text(build_info)

    # Create the zipapp
    zipapp.create_archive(
        p,
        'xd.pyz',
        '/usr/bin/env python3',
        'xd:main'
    )
EOF

# Make executable
chmod +x xd.pyz

# Show result
SIZE=$(ls -lh xd.pyz | awk '{print $5}')
echo "Done: xd.pyz ($SIZE)"

# Install if requested
if [ "$INSTALL" = true ]; then
    INSTALL_DIR="$HOME/.local/bin"
    
    # Create install directory if it doesn't exist
    if [ ! -d "$INSTALL_DIR" ]; then
        echo "Creating install directory: $INSTALL_DIR"
        mkdir -p "$INSTALL_DIR"
    fi
    
    # Install
    echo "Installing to $INSTALL_DIR/xd..."
    cp xd.pyz "$INSTALL_DIR/xd"
    chmod +x "$INSTALL_DIR/xd"
    
    echo ""
    echo "Installation complete!"
    echo "Run 'xd --help' to use the installed version."
fi

# Verify
echo ""
echo "Contents:"
unzip -l xd.pyz | grep -v "__pycache__" | tail -5