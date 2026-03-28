#!/bin/bash
# xdotter bubblewrap 隔离测试
# 使用 bubblewrap 创建轻量级隔离环境测试 dotfiles 部署

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
TEST_DIR="/tmp/xdotter-bwrap-test"
ISOLATED_HOME="$TEST_DIR/home"
DOTFILES_DIR="$TEST_DIR/dotfiles"

echo "╔════════════════════════════════════════════════════════╗"
echo "║  xdotter Bubblewrap 隔离测试                            ║"
echo "╚════════════════════════════════════════════════════════╝"
echo ""

# 清理函数
cleanup() {
    echo ""
    echo "[清理] 删除临时目录..."
    rm -rf "$TEST_DIR"
}
trap cleanup EXIT

# 步骤 1: 准备隔离环境
echo "[1/6] 准备隔离环境..."

mkdir -p "$ISOLATED_HOME/.config"
mkdir -p "$ISOLATED_HOME/.local/share"
mkdir -p "$DOTFILES_DIR"

# 克隆或复制 dotfiles
if [ -d "$SCRIPT_DIR/test-dotfiles" ]; then
    cp -r "$SCRIPT_DIR/test-dotfiles/"* "$DOTFILES_DIR/"
else
    git clone --depth=1 https://github.com/cncsmonster/dotfiles.git "$DOTFILES_DIR" 2>/dev/null
fi

echo "  → 隔离 HOME: $ISOLATED_HOME"
echo "  → Dotfiles:  $DOTFILES_DIR"

# 步骤 2: Dry-run 测试
echo ""
echo "[2/6] Dry-run 测试（配置解析验证）..."
echo ""

bwrap \
    --ro-bind "$SCRIPT_DIR/xd.py" /app/xd.py \
    --ro-bind "$DOTFILES_DIR" /tmp/dotfiles \
    --bind "$ISOLATED_HOME" "$ISOLATED_HOME" \
    --ro-bind /usr /usr \
    --ro-bind /etc /etc \
    --ro-bind /lib /lib \
    --ro-bind /lib64 /lib64 \
    --setenv HOME "$ISOLATED_HOME" \
    --chdir /tmp/dotfiles \
    python3 /app/xd.py deploy -n -v 2>&1 | tee "$SCRIPT_DIR/bwrap-dry-run.log"

echo ""
echo "  → Dry-run 完成"

# 步骤 3: 实际部署
echo ""
echo "[3/6] 实际部署..."
echo ""

bwrap \
    --ro-bind "$SCRIPT_DIR/xd.py" /app/xd.py \
    --ro-bind "$DOTFILES_DIR" /tmp/dotfiles \
    --bind "$ISOLATED_HOME" "$ISOLATED_HOME" \
    --ro-bind /usr /usr \
    --ro-bind /etc /etc \
    --ro-bind /lib /lib \
    --ro-bind /lib64 /lib64 \
    --setenv HOME "$ISOLATED_HOME" \
    --chdir /tmp/dotfiles \
    python3 /app/xd.py deploy -v 2>&1 | tee "$SCRIPT_DIR/bwrap-deploy.log"

echo ""
echo "  → 部署完成"

# 步骤 4: 验证部署结果
echo ""
echo "[4/6] 验证部署结果..."
echo ""

echo "  主要 symlinks:"
echo "  ─────────────────────────────────────"
ls -la "$ISOLATED_HOME/.config/" | grep -E "^l" | head -15
echo "  ─────────────────────────────────────"

# 验证关键链接
verify_link() {
    local path="$1"
    local name="$2"
    if [ -L "$path" ]; then
        target=$(readlink "$path")
        echo "  ✓ $name → $target"
        return 0
    else
        echo "  ✗ $name (未创建)"
        return 1
    fi
}

echo ""
echo "  详细验证:"
verify_link "$ISOLATED_HOME/.config/yazi" "yazi"
verify_link "$ISOLATED_HOME/.config/git" "git"
verify_link "$ISOLATED_HOME/.config/starship.toml" "starship.toml"
verify_link "$ISOLATED_HOME/.local/share/navi/cheats" "navi/cheats"

# 验证子目录部署
echo ""
echo "  子目录部署验证:"
verify_link "$ISOLATED_HOME/.config/shells/common" "shells/common (from shells/xdotter.toml)"
verify_link "$ISOLATED_HOME/.config/nvims/LazyVim" "nvims/LazyVim (from nvims/xdotter.toml)"
verify_link "$ISOLATED_HOME/.zshrc" ".zshrc (from shells/zsh/xdotter.toml)"
verify_link "$ISOLATED_HOME/.bashrc" ".bashrc (from shells/bash/xdotter.toml)"
verify_link "$ISOLATED_HOME/.cargo/config.toml" ".cargo/config.toml (from langs/rust/cargo/xdotter.toml)"

# 步骤 5: 测试文件内容
echo ""
echo "[5/6] 验证 symlink 内容可访问..."

# 注意：symlink 指向 /tmp/dotfiles，在清理前应该可访问
test_file="$ISOLATED_HOME/.config/starship.toml"
if [ -L "$test_file" ]; then
    target=$(readlink "$test_file")
    if [ -f "$target" ]; then
        lines=$(wc -l < "$target")
        echo "  ✓ starship.toml 可读 ($lines 行)"
    else
        echo "  ! starship.toml 源文件不可读 (正常，dotfiles 已清理)"
    fi
else
    echo "  ✗ starship.toml symlink 不存在"
fi

# 步骤 6: Undeploy 测试
echo ""
echo "[6/6] Undeploy 测试..."
echo ""

bwrap \
    --ro-bind "$SCRIPT_DIR/xd.py" /app/xd.py \
    --ro-bind "$DOTFILES_DIR" /tmp/dotfiles \
    --bind "$ISOLATED_HOME" "$ISOLATED_HOME" \
    --ro-bind /usr /usr \
    --ro-bind /etc /etc \
    --ro-bind /lib /lib \
    --ro-bind /lib64 /lib64 \
    --setenv HOME "$ISOLATED_HOME" \
    --chdir /tmp/dotfiles \
    python3 /app/xd.py undeploy -v 2>&1 | tee "$SCRIPT_DIR/bwrap-undeploy.log"

echo ""
echo "  验证清理:"
remaining=$(find "$ISOLATED_HOME" -type l 2>/dev/null | wc -l)
if [ "$remaining" -eq 0 ]; then
    echo "  ✓ 所有 symlinks 已清理"
else
    echo "  ! 残留 $remaining 个 symlinks"
    find "$ISOLATED_HOME" -type l 2>/dev/null
fi

echo ""
echo "╔════════════════════════════════════════════════════════╗"
echo "║  Bubblewrap 测试完成！                                  ║"
echo "╚════════════════════════════════════════════════════════╝"
echo ""
echo "日志文件:"
echo "  - Dry-run:   $SCRIPT_DIR/bwrap-dry-run.log"
echo "  - Deploy:    $SCRIPT_DIR/bwrap-deploy.log"
echo "  - Undeploy:  $SCRIPT_DIR/bwrap-undeploy.log"
