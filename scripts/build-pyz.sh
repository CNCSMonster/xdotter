#!/bin/bash
# Build xd.pyz - single-file executable Python archive
# Usage: ./scripts/build-pyz.sh

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"

cd "$PROJECT_DIR"

echo "Building xd.pyz..."

python3 -c "
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
    
    # Create the zipapp
    zipapp.create_archive(
        p, 
        'xd.pyz', 
        '/usr/bin/env python3', 
        'xd:main'
    )
"

# Make executable
chmod +x xd.pyz

# Show result
SIZE=$(ls -lh xd.pyz | awk '{print $5}')
echo "Done: xd.pyz ($SIZE)"

# Verify
echo ""
echo "Contents:"
unzip -l xd.pyz | grep -v "__pycache__" | tail -5