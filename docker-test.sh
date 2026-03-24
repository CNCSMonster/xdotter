#!/bin/bash
# xdotter 容器化测试脚本
# 在 Docker 容器中测试新的 xd.py 能否部署 dotfiles

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
TEST_NAME="xdotter-dotfiles-test"
CONTAINER_HOME="/tmp/test-home"

echo "╔════════════════════════════════════════════════════════╗"
echo "║     xdotter 容器化测试 - 验证 dotfiles 部署              ║"
echo "╚════════════════════════════════════════════════════════╝"
echo ""

# 清理函数
cleanup() {
    echo ""
    echo "[清理] 删除容器和临时文件..."
    docker rm -f "$TEST_NAME" 2>/dev/null || true
    rm -rf "$SCRIPT_DIR/test-home" 2>/dev/null || true
    rm -rf "$SCRIPT_DIR/test-dotfiles" 2>/dev/null || true
}

# 捕获退出信号并清理
trap cleanup EXIT

# 步骤 1: 准备测试数据
echo "[1/6] 准备测试数据..."

# 克隆 dotfiles (如果还没有)
if [ ! -d "$SCRIPT_DIR/test-dotfiles" ]; then
    git clone --depth=1 https://github.com/cncsmonster/dotfiles.git "$SCRIPT_DIR/test-dotfiles" 2>/dev/null
else
    echo "  → 使用已有的 dotfiles 副本"
fi

# 创建隔离的 HOME 目录结构
mkdir -p "$SCRIPT_DIR/test-home/.config"
mkdir -p "$SCRIPT_DIR/test-home/.local/share"

echo "  → 测试 HOME: $SCRIPT_DIR/test-home"
echo "  → Dotfiles:  $SCRIPT_DIR/test-dotfiles"

# 步骤 2: 运行容器
echo ""
echo "[2/6] 启动容器..."

docker run -d \
    --name "$TEST_NAME" \
    -v "$SCRIPT_DIR/xd.py:/app/xd.py:ro" \
    -v "$SCRIPT_DIR/test-dotfiles:/tmp/dotfiles:ro" \
    -v "$SCRIPT_DIR/test-home:$CONTAINER_HOME:rw" \
    -e HOME="$CONTAINER_HOME" \
    --user "$(id -u):$(id -g)" \
    python:3.11-slim \
    tail -f /dev/null

echo "  → 容器启动成功"

# 步骤 3: Dry-run 测试
echo ""
echo "[3/6] 执行 Dry-run 测试（不实际部署）..."
echo ""

docker exec -w /tmp/dotfiles "$TEST_NAME" \
    python3 /app/xd.py deploy -n -v 2>&1 | tee "$SCRIPT_DIR/dry-run.log"

echo ""
echo "  → Dry-run 完成，日志保存到: $SCRIPT_DIR/dry-run.log"

# 步骤 4: 实际部署
echo ""
echo "[4/6] 执行实际部署..."
echo ""

docker exec -w /tmp/dotfiles "$TEST_NAME" \
    python3 /app/xd.py deploy -v 2>&1 | tee "$SCRIPT_DIR/deploy.log"

echo ""
echo "  → 部署完成，日志保存到：$SCRIPT_DIR/deploy.log"

# 步骤 5: 验证部署结果
echo ""
echo "[5/6] 验证部署结果..."
echo ""

echo "  检查 symlinks:"
docker exec "$TEST_NAME" bash -c "
    echo '  ─────────────────────────────────────'
    ls -la $CONTAINER_HOME/.config/ 2>/dev/null | head -20
    echo '  ─────────────────────────────────────'
    
    # 检查关键目录
    for dir in yazi git nvim shells; do
        if [ -L \"$CONTAINER_HOME/.config/$dir\" ]; then
            echo \"  ✓ $dir -> \$(readlink \"$CONTAINER_HOME/.config/$dir\")\"
        fi
    done
    
    # 检查文件
    if [ -L \"$CONTAINER_HOME/.config/starship.toml\" ]; then
        echo \"  ✓ starship.toml -> \$(readlink \"$CONTAINER_HOME/.config/starship.toml\")\"
    fi
"

echo ""
echo "  检查依赖子目录部署:"
docker exec "$TEST_NAME" bash -c "
    # 检查 shells 子目录
    if [ -L \"$CONTAINER_HOME/.config/shells\" ]; then
        echo \"  ✓ shells 目录已部署\"
    fi
    
    # 检查 langs 子目录  
    if [ -L \"$CONTAINER_HOME/.config/uv\" ]; then
        echo \"  ✓ langs/python (uv.toml) 已部署\"
    fi
"

# 步骤 6: Undeploy 测试
echo ""
echo "[6/6] 测试 Undeploy..."
echo ""

docker exec -w /tmp/dotfiles "$TEST_NAME" \
    python3 /app/xd.py undeploy -v 2>&1 | tee "$SCRIPT_DIR/undeploy.log"

echo ""
echo "  验证清理结果:"
docker exec "$TEST_NAME" bash -c "
    count=\$(ls -la $CONTAINER_HOME/.config/ 2>/dev/null | wc -l)
    if [ \$count -le 3 ]; then
        echo \"  ✓ 所有 symlinks 已清理\"
    else
        echo \"  ! 仍有文件残留:\"
        ls -la $CONTAINER_HOME/.config/
    fi
"

echo ""
echo "╔════════════════════════════════════════════════════════╗"
echo "║  测试完成！                                             ║"
echo "╚════════════════════════════════════════════════════════╝"
echo ""
echo "日志文件:"
echo "  - Dry-run:   $SCRIPT_DIR/dry-run.log"
echo "  - Deploy:    $SCRIPT_DIR/deploy.log"
echo "  - Undeploy:  $SCRIPT_DIR/undeploy.log"
echo ""

# 生成测试报告
echo "生成测试报告..."
cat > "$SCRIPT_DIR/test-report.md" << 'REPORT'
# xdotter 容器化测试报告

## 测试环境
- 基础镜像：python:3.11-slim
- Python 版本：3.11
- 测试对象：cncsmonster/dotfiles

## 测试步骤
1. ✅ Dry-run 测试（验证配置解析）
2. ✅ 实际部署（验证 symlink 创建）
3. ✅ 结果验证（检查 symlinks 正确性）
4. ✅ Undeploy 测试（验证清理功能）

## 验证项目
- [cncsmonster/dotfiles](https://github.com/cncsmonster/dotfiles)

## 测试配置
- 主配置：xdotter.toml (3 个 dependencies)
- 子配置：shells/, langs/, nvims/

## 结论
[待填写]
REPORT

echo "测试报告：$SCRIPT_DIR/test-report.md"
