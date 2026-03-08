#!/bin/bash
# Script to vendor tomli library into xdotter project
# Usage: ./scripts/vendor-tomli.sh [version]

set -e

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Configuration
VENDOR_DIR="_vendor"
TOMLI_DIR="$VENDOR_DIR/tomli"
VERSION=${1:-"latest"}

info() {
    echo -e "${GREEN}✓${NC} $1"
}

warn() {
    echo -e "${YELLOW}!${NC} $1"
}

error() {
    echo -e "${RED}✗${NC} $1"
}

# Get version number
get_version() {
    if [ "$VERSION" = "latest" ]; then
        VERSION=$(curl -sSL https://pypi.org/pypi/tomli/json | python3 -c "import sys, json; print(json.load(sys.stdin)['info']['version'])")
        info "Latest version: $VERSION"
    else
        info "Using specified version: $VERSION"
    fi
}

# Create vendor directory
setup_vendor_dir() {
    info "Setting up vendor directory..."
    
    # Remove old version if exists
    if [ -d "$TOMLI_DIR" ]; then
        warn "Removing old version..."
        rm -rf "$TOMLI_DIR"
    fi
    
    # Create directory structure
    mkdir -p "$TOMLI_DIR"
}

# Download tomli source files
download_tomli() {
    info "Downloading tomli $VERSION source files..."
    
    BASE_URL="https://raw.githubusercontent.com/hukkin/tomli/v$VERSION/src/tomli"
    
    # Download main files
    files=(
        "__init__.py"
        "_parser.py"
        "_re.py"
        "_types.py"
    )
    
    for file in "${files[@]}"; do
        echo "  Downloading $file..."
        curl -sSL "$BASE_URL/$file" > "$TOMLI_DIR/$file"
    done
    
    # Download LICENSE
    echo "  Downloading LICENSE..."
    curl -sSL "https://raw.githubusercontent.com/hukkin/tomli/v$VERSION/LICENSE" > "$TOMLI_DIR/LICENSE"
}

# Create __init__.py for vendor directory
create_vendor_init() {
    info "Creating vendor __init__.py..."
    
    cat > "$VENDOR_DIR/__init__.py" << 'EOF'
"""
Vendored third-party libraries for xdotter.

These libraries are embedded to maintain zero external dependencies.
Each library retains its original license.
"""
EOF
}

# Create README for vendor directory
create_vendor_readme() {
    info "Creating vendor README..."
    
    cat > "$VENDOR_DIR/README.md" << EOF
# Vendored Libraries

This directory contains vendored third-party libraries used by xdotter.

## Libraries

### tomli

- **Version**: $VERSION
- **Author**: Taneli Hukkinen
- **License**: MIT
- **Source**: https://github.com/hukkin/tomli
- **Purpose**: TOML parsing for Python < 3.11

## License

Each library retains its original license. See the LICENSE file in each subdirectory.

## Updating

To update a vendored library, run:

\`\`\`bash
./scripts/vendor-tomli.sh [version]
\`\`\`
EOF
}

# Update .gitignore
update_gitignore() {
    info "Updating .gitignore..."
    
    if ! grep -q "_vendor/" .gitignore 2>/dev/null; then
        cat >> .gitignore << 'EOF'

# Vendored libraries
_vendor/
!_vendor/.gitkeep
!_vendor/**/LICENSE
!_vendor/**/README.md
EOF
        info "Updated .gitignore"
    else
        warn ".gitignore already contains vendor rules"
    fi
    
    # Create .gitkeep
    if [ ! -f "$VENDOR_DIR/.gitkeep" ]; then
        touch "$VENDOR_DIR/.gitkeep"
    fi
}

# Verify download
verify_download() {
    info "Verifying download..."
    
    required_files=(
        "$TOMLI_DIR/__init__.py"
        "$TOMLI_DIR/_parser.py"
        "$TOMLI_DIR/_re.py"
        "$TOMLI_DIR/_types.py"
        "$TOMLI_DIR/LICENSE"
    )
    
    all_exist=true
    for file in "${required_files[@]}"; do
        if [ ! -f "$file" ]; then
            error "Missing file: $file"
            all_exist=false
        fi
    done
    
    if [ "$all_exist" = true ]; then
        info "✓ All files downloaded successfully"
        
        # Count lines
        total_lines=$(cat "$TOMLI_DIR"/*.py | wc -l)
        info "Total Python code: $total_lines lines"
        
        # Show file sizes
        info "Directory size:"
        du -h "$TOMLI_DIR"
    else
        error "Download verification failed!"
        exit 1
    fi
}

# Print next steps
print_next_steps() {
    echo ""
    info "Vendoring complete!"
    echo ""
    echo "Next steps:"
    echo "  1. Modify xd.py to use vendored tomli:"
    echo "     "
    echo "     # Add at the top of xd.py:"
    echo "     import sys"
    echo "     from pathlib import Path"
    echo "     _vendor_path = Path(__file__).parent / '_vendor'"
    echo "     if str(_vendor_path) not in sys.path:"
    echo "         sys.path.insert(0, str(_vendor_path))"
    echo "     "
    echo "     # Replace ConfigParser.parse() with:"
    echo "     from tomli import loads"
    echo "     def parse(content: str) -> Dict:"
    echo "         raw_data = loads(content)"
    echo "         return {"
    echo "             'links': raw_data.get('links', {}),"
    echo "             'dependencies': raw_data.get('dependencies', {})"
    echo "         }"
    echo ""
    echo "  2. Run tests to verify:"
    echo "     python test_xd.py"
    echo ""
    echo "  3. Commit changes:"
    echo "     git add _vendor/"
    echo "     git commit -m 'chore: vendor tomli $VERSION for robust TOML parsing'"
    echo ""
}

# Main
main() {
    echo "╔══════════════════════════════════════╗"
    echo "║     Vendoring tomli into xdotter     ║"
    echo "╚══════════════════════════════════════╝"
    echo ""
    
    get_version
    setup_vendor_dir
    download_tomli
    create_vendor_init
    create_vendor_readme
    update_gitignore
    verify_download
    print_next_steps
}

main "$@"
