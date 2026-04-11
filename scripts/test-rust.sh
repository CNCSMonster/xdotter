#!/bin/bash
# Test runner for Rust xdotter

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
RUST_BIN="$PROJECT_DIR/target/debug/xd"

PASSED=0
FAILED=0

log_test() {
    local name="$1"
    local status="$2"
    local message="${3:-}"
    
    if [ "$status" = "PASS" ]; then
        echo "  [PASS] $name"
        PASSED=$((PASSED + 1))
    elif [ "$status" = "SKIP" ]; then
        echo "  [SKIP] $name ${message}"
    else
        echo "  [FAIL] $name ${message}"
        FAILED=$((FAILED + 1))
    fi
}

run_xd() {
    local home_dir="$1"
    shift
    (export HOME="$home_dir"; "$RUST_BIN" "$@" 2>&1)
}

setup_tmp() {
    local tmpdir=$(mktemp -d)
    mkdir -p "$tmpdir/source" "$tmpdir/.cache"
    echo "test content" > "$tmpdir/source/test.txt"
    cat > "$tmpdir/xdotter.toml" << 'EOF'
[links]
"source/test.txt" = "~/.cache/xdotter_test.txt"
EOF
    echo "$tmpdir"
}

echo "=================================================="
echo "xdotter Rust Test Suite"
echo "=================================================="

# Test: Help Command
tmpdir=$(setup_tmp)
output=$(run_xd "$tmpdir" --help)
if echo "$output" | grep -q "deploy" && echo "$output" | grep -q "undeploy"; then
    log_test "Help Command" "PASS"
else
    log_test "Help Command" "FAIL" "missing commands"
fi
rm -rf "$tmpdir"

# Test: Version Command
tmpdir=$(setup_tmp)
output=$(run_xd "$tmpdir" version)
if echo "$output" | grep -qE "^[0-9]+\.[0-9]+\.[0-9]+"; then
    log_test "Version Command" "PASS"
else
    log_test "Version Command" "FAIL" "output: $output"
fi
rm -rf "$tmpdir"

# Test: New Command
tmpdir=$(mktemp -d)
run_xd "$tmpdir" new > /dev/null 2>&1
if [ -f "$tmpdir/xdotter.toml" ] && grep -q "\[links\]" "$tmpdir/xdotter.toml"; then
    log_test "New Command" "PASS"
else
    log_test "New Command" "FAIL" "file not created"
fi
rm -rf "$tmpdir"

# Test: Config Parsing
tmpdir=$(setup_tmp)
output=$(run_xd "$tmpdir" -v deploy)
if echo "$output" | grep -q "deploy:"; then
    log_test "Config Parsing" "PASS"
else
    log_test "Config Parsing" "FAIL" "output: $output"
fi
rm -rf "$tmpdir"

# Test: Deploy Basic Link
tmpdir=$(setup_tmp)
run_xd "$tmpdir" deploy > /dev/null 2>&1
if [ -L "$tmpdir/.cache/xdotter_test.txt" ]; then
    log_test "Deploy Basic Link" "PASS"
else
    log_test "Deploy Basic Link" "FAIL" "symlink not created"
fi
rm -rf "$tmpdir"

# Test: Deploy Dry Run
tmpdir=$(setup_tmp)
run_xd "$tmpdir" -n deploy > /dev/null 2>&1
if [ ! -e "$tmpdir/.cache/xdotter_test.txt" ]; then
    log_test "Deploy Dry Run" "PASS"
else
    log_test "Deploy Dry Run" "FAIL"
fi
rm -rf "$tmpdir"

# Test: Tilde Expansion
tmpdir=$(setup_tmp)
cat > "$tmpdir/xdotter.toml" << EOF
[links]
"source/test.txt" = "~/.cache/xdotter_tilde_test.txt"
EOF
run_xd "$tmpdir" deploy > /dev/null 2>&1
if [ -L "$tmpdir/.cache/xdotter_tilde_test.txt" ]; then
    log_test "Tilde Expansion" "PASS"
else
    log_test "Tilde Expansion" "FAIL"
fi
rm -rf "$tmpdir"

# Test: Multiple Links
tmpdir=$(setup_tmp)
echo "file2" > "$tmpdir/source/test2.txt"
cat > "$tmpdir/xdotter.toml" << EOF
[links]
"source/test.txt" = "~/.cache/xdotter_multi1.txt"
"source/test2.txt" = "~/.cache/xdotter_multi2.txt"
EOF
run_xd "$tmpdir" deploy > /dev/null 2>&1
if [ -L "$tmpdir/.cache/xdotter_multi1.txt" ] && [ -L "$tmpdir/.cache/xdotter_multi2.txt" ]; then
    log_test "Multiple Links" "PASS"
else
    log_test "Multiple Links" "FAIL"
fi
rm -rf "$tmpdir"

# Test: Undeploy
tmpdir=$(setup_tmp)
run_xd "$tmpdir" deploy > /dev/null 2>&1
if [ -L "$tmpdir/.cache/xdotter_test.txt" ]; then
    run_xd "$tmpdir" undeploy > /dev/null 2>&1
    if [ ! -e "$tmpdir/.cache/xdotter_test.txt" ]; then
        log_test "Undeploy" "PASS"
    else
        log_test "Undeploy" "FAIL" "symlink not removed"
    fi
else
    log_test "Undeploy" "FAIL" "precondition failed"
fi
rm -rf "$tmpdir"

# Test: Quiet Mode
tmpdir=$(setup_tmp)
output=$(run_xd "$tmpdir" -q deploy)
if [ -z "$output" ]; then
    log_test "Quiet Mode" "PASS"
else
    log_test "Quiet Mode" "FAIL" "got output: $output"
fi
rm -rf "$tmpdir"

# Test: Verbose Mode
tmpdir=$(setup_tmp)
output=$(run_xd "$tmpdir" -v deploy)
if echo "$output" | grep -q "\[DEBUG\]"; then
    log_test "Verbose Mode" "PASS"
else
    log_test "Verbose Mode" "FAIL"
fi
rm -rf "$tmpdir"

# Test: Force Flag
tmpdir=$(setup_tmp)
run_xd "$tmpdir" deploy > /dev/null 2>&1
rm -f "$tmpdir/.cache/xdotter_test.txt"
echo "regular file" > "$tmpdir/.cache/xdotter_test.txt"
run_xd "$tmpdir" -f deploy > /dev/null 2>&1
if [ -L "$tmpdir/.cache/xdotter_test.txt" ]; then
    log_test "Force Flag" "PASS"
else
    log_test "Force Flag" "FAIL"
fi
rm -rf "$tmpdir"

# Test: Nonexistent Source
tmpdir=$(mktemp -d)
cat > "$tmpdir/xdotter.toml" << 'EOF'
[links]
"source/nonexistent.txt" = "~/.cache/xdotter_noexist.txt"
EOF
output=$(run_xd "$tmpdir" deploy)
if echo "$output" | grep -qi "error\|does not exist"; then
    log_test "Nonexistent Source" "PASS"
else
    log_test "Nonexistent Source" "FAIL"
fi
rm -rf "$tmpdir"

# Test: Nonexistent Config
tmpdir=$(mktemp -d)
output=$(run_xd "$tmpdir" deploy)
if echo "$output" | grep -qi "error\|not found"; then
    log_test "Nonexistent Config" "PASS"
else
    log_test "Nonexistent Config" "FAIL"
fi
rm -rf "$tmpdir"

# Test: Invalid TOML
tmpdir=$(mktemp -d)
echo "[links" > "$tmpdir/xdotter.toml"
output=$(run_xd "$tmpdir" deploy)
if echo "$output" | grep -qi "error\|validation\|syntax"; then
    log_test "Invalid TOML" "PASS"
else
    log_test "Invalid TOML" "FAIL"
fi
rm -rf "$tmpdir"

# Test: Empty Config
tmpdir=$(mktemp -d)
echo "# empty" > "$tmpdir/xdotter.toml"
output=$(run_xd "$tmpdir" deploy)
if [ $? -eq 0 ]; then
    log_test "Empty Config" "PASS"
else
    log_test "Empty Config" "FAIL"
fi
rm -rf "$tmpdir"

# Test: Symlink Already Exists
tmpdir=$(setup_tmp)
run_xd "$tmpdir" deploy > /dev/null 2>&1
output=$(run_xd "$tmpdir" -v deploy)
if echo "$output" | grep -qi "skip\|already"; then
    log_test "Symlink Already Exists" "PASS"
else
    log_test "Symlink Already Exists" "FAIL"
fi
rm -rf "$tmpdir"

# Test: Unicode Paths
tmpdir=$(mktemp -d)
mkdir -p "$tmpdir/source"
echo "unicode" > "$tmpdir/source/测试.txt"
cat > "$tmpdir/xdotter.toml" << 'EOF'
[links]
"source/测试.txt" = "~/.cache/xdotter_unicode_测试.txt"
EOF
run_xd "$tmpdir" deploy > /dev/null 2>&1
if [ -L "$tmpdir/.cache/xdotter_unicode_测试.txt" ]; then
    log_test "Unicode Paths" "PASS"
else
    log_test "Unicode Paths" "FAIL"
fi
rm -rf "$tmpdir"

# Test: Absolute Path In Config
tmpdir=$(mktemp -d)
mkdir -p "$tmpdir/source" "$tmpdir/.cache"
echo "absolute" > "$tmpdir/source/test.txt"
abs_target="$tmpdir/.cache/xdotter_abs.txt"
cat > "$tmpdir/xdotter.toml" << EOF
[links]
"$tmpdir/source/test.txt" = "$abs_target"
EOF
run_xd "$tmpdir" deploy > /dev/null 2>&1
if [ -L "$abs_target" ]; then
    log_test "Absolute Path In Config" "PASS"
else
    log_test "Absolute Path In Config" "FAIL"
fi
rm -rf "$tmpdir"

# Test: Undeploy Nonexistent Link
tmpdir=$(mktemp -d)
cat > "$tmpdir/xdotter.toml" << 'EOF'
[links]
"source/test.txt" = "~/.cache/xdotter_noexist_undeploy.txt"
EOF
output=$(run_xd "$tmpdir" undeploy)
if [ $? -eq 0 ]; then
    log_test "Undeploy Nonexistent Link" "PASS"
else
    log_test "Undeploy Nonexistent Link" "FAIL"
fi
rm -rf "$tmpdir"

# Test: Comments In Config
tmpdir=$(mktemp -d)
mkdir -p "$tmpdir/source" "$tmpdir/.cache"
echo "commented" > "$tmpdir/source/test.txt"
cat > "$tmpdir/xdotter.toml" << 'EOF'
# This is a comment
[links]
# Another comment
"source/test.txt" = "~/.cache/xdotter_comment.txt"
EOF
run_xd "$tmpdir" deploy > /dev/null 2>&1
if [ -L "$tmpdir/.cache/xdotter_comment.txt" ]; then
    log_test "Comments In Config" "PASS"
else
    log_test "Comments In Config" "FAIL"
fi
rm -rf "$tmpdir"

# Test: Validate Valid TOML
tmpdir=$(mktemp -d)
cat > "$tmpdir/xdotter.toml" << 'EOF'
[links]
"source/test.txt" = "~/.cache/test.txt"
EOF
output=$(run_xd "$tmpdir" validate)
if echo "$output" | grep -qi "valid"; then
    log_test "Validate Valid TOML" "PASS"
else
    log_test "Validate Valid TOML" "FAIL"
fi
rm -rf "$tmpdir"

# Test: Validate Invalid TOML
tmpdir=$(mktemp -d)
echo "[links" > "$tmpdir/xdotter.toml"
output=$(run_xd "$tmpdir" validate)
if echo "$output" | grep -qi "error\|invalid\|fail"; then
    log_test "Validate Invalid TOML" "PASS"
else
    log_test "Validate Invalid TOML" "FAIL"
fi
rm -rf "$tmpdir"

# Test: Validate Valid JSON
tmpdir=$(mktemp -d)
echo '{"links": {"source/test.txt": "~/.cache/test.txt"}}' > "$tmpdir/xdotter.json"
output=$(run_xd "$tmpdir" validate "$tmpdir/xdotter.json")
if echo "$output" | grep -qi "valid"; then
    log_test "Validate Valid JSON" "PASS"
else
    log_test "Validate Valid JSON" "FAIL"
fi
rm -rf "$tmpdir"

# Test: Validate Invalid JSON
tmpdir=$(mktemp -d)
echo '{"links": }' > "$tmpdir/xdotter.json"
output=$(run_xd "$tmpdir" validate "$tmpdir/xdotter.json")
if echo "$output" | grep -qi "error\|invalid\|fail"; then
    log_test "Validate Invalid JSON" "PASS"
else
    log_test "Validate Invalid JSON" "FAIL"
fi
rm -rf "$tmpdir"

# Test: Completion Bash
tmpdir=$(mktemp -d)
output=$(run_xd "$tmpdir" completion bash)
if echo "$output" | grep -q "xd"; then
    log_test "Completion Bash" "PASS"
else
    log_test "Completion Bash" "FAIL"
fi
rm -rf "$tmpdir"

# Test: Completion Zsh
tmpdir=$(mktemp -d)
output=$(run_xd "$tmpdir" completion zsh)
if echo "$output" | grep -q "xd"; then
    log_test "Completion Zsh" "PASS"
else
    log_test "Completion Zsh" "FAIL"
fi
rm -rf "$tmpdir"

# Test: Completion Fish
tmpdir=$(mktemp -d)
output=$(run_xd "$tmpdir" completion fish)
if echo "$output" | grep -q "xd"; then
    log_test "Completion Fish" "PASS"
else
    log_test "Completion Fish" "FAIL"
fi
rm -rf "$tmpdir"

# Test: Completion No Shell
tmpdir=$(mktemp -d)
output=$(run_xd "$tmpdir" completion 2>&1)
if echo "$output" | grep -qi "error\|required\|missing"; then
    log_test "Completion No Shell" "PASS"
else
    log_test "Completion No Shell" "FAIL"
fi
rm -rf "$tmpdir"

# Test: Completion Invalid Shell
tmpdir=$(mktemp -d)
output=$(run_xd "$tmpdir" completion invalid_shell 2>&1)
if echo "$output" | grep -qi "error\|unsupported\|invalid"; then
    log_test "Completion Invalid Shell" "PASS"
else
    log_test "Completion Invalid Shell" "FAIL"
fi
rm -rf "$tmpdir"

# Test: Deploy Auto-Validation Invalid
tmpdir=$(mktemp -d)
echo "[links" > "$tmpdir/xdotter.toml"
output=$(run_xd "$tmpdir" deploy)
if echo "$output" | grep -qi "error\|validation\|fail"; then
    log_test "Deploy Auto-Validation Invalid" "PASS"
else
    log_test "Deploy Auto-Validation Invalid" "FAIL"
fi
rm -rf "$tmpdir"

# Test: Deploy No Validate Flag
tmpdir=$(mktemp -d)
mkdir -p "$tmpdir/source" "$tmpdir/.cache"
echo "test" > "$tmpdir/source/test.txt"
cat > "$tmpdir/xdotter.toml" << 'EOF'
[links]
"source/test.txt" = "~/.cache/test.txt"
EOF
output=$(run_xd "$tmpdir" --no-validate deploy)
if [ $? -eq 0 ]; then
    log_test "Deploy No Validate Flag" "PASS"
else
    log_test "Deploy No Validate Flag" "FAIL"
fi
rm -rf "$tmpdir"

# Test: Deploy Auto-Validation Valid
tmpdir=$(setup_tmp)
output=$(run_xd "$tmpdir" deploy)
if [ $? -eq 0 ]; then
    log_test "Deploy Auto-Validation Valid" "PASS"
else
    log_test "Deploy Auto-Validation Valid" "FAIL"
fi
rm -rf "$tmpdir"

# Test: Symlink Loop Detection
tmpdir=$(mktemp -d)
mkdir -p "$tmpdir/.config" "$tmpdir/dotfiles/.config"
ln -s "$tmpdir/dotfiles/.config" "$tmpdir/.config"
echo "test" > "$tmpdir/dotfiles/.config/file.txt"
cat > "$tmpdir/xdotter.toml" << EOF
[links]
"$tmpdir/dotfiles/.config/file.txt" = "$tmpdir/.config/file.txt"
EOF
output=$(run_xd "$tmpdir" -v deploy)
if echo "$output" | grep -qi "loop\|skip\|warning"; then
    log_test "Symlink Loop Detection" "PASS"
else
    log_test "Symlink Loop Detection" "FAIL" "output: $output"
fi
rm -rf "$tmpdir"

# Test: Circular Symlink Scenario
tmpdir=$(mktemp -d)
mkdir -p "$tmpdir/A"
echo "test" > "$tmpdir/A/file.txt"
ln -s "$tmpdir/A" "$tmpdir/C"
cat > "$tmpdir/xdotter.toml" << EOF
[links]
"$tmpdir/A/file.txt" = "$tmpdir/C/file.txt"
EOF
output=$(run_xd "$tmpdir" -v deploy)
if echo "$output" | grep -qi "circular\|skip\|warning"; then
    log_test "Circular Symlink Scenario" "PASS"
else
    log_test "Circular Symlink Scenario" "FAIL" "output: $output"
fi
rm -rf "$tmpdir"

# Test: Force Fixes Parent Symlink
tmpdir=$(mktemp -d)
mkdir -p "$tmpdir/dotfiles/helix" "$tmpdir/.config"
echo "source content" > "$tmpdir/dotfiles/helix/config.toml"
ln -s "../dotfiles/helix" "$tmpdir/.config/helix"
cat > "$tmpdir/xdotter.toml" << EOF
[links]
"$tmpdir/dotfiles/helix/config.toml" = "$tmpdir/.config/helix/config.toml"
EOF
output=$(run_xd "$tmpdir" -f -v deploy)
if [ -d "$tmpdir/.config/helix" ] && [ ! -L "$tmpdir/.config/helix" ]; then
    if [ -L "$tmpdir/.config/helix/config.toml" ]; then
        log_test "Force Fixes Parent Symlink" "PASS"
    else
        log_test "Force Fixes Parent Symlink" "FAIL" "symlink not created"
    fi
else
    log_test "Force Fixes Parent Symlink" "FAIL" "parent symlink not replaced"
fi
rm -rf "$tmpdir"

# Summary
echo ""
echo "=================================================="
echo "Test Summary: $PASSED/$((PASSED + FAILED)) passed"
echo "  Passed:  $PASSED"
echo "  Failed:  $FAILED"
echo "=================================================="

if [ $FAILED -gt 0 ]; then
    exit 1
fi
