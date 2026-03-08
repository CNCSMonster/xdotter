#!/usr/bin/env bash
# 在隔离环境中运行 test_xd.py（bwrap 模拟新环境，不触碰真实 HOME）
# 用法: ./scripts/run-tests-isolated.sh
#
# 依赖: bwrap (bubblewrap)，常见于 Linux；无 bwrap 时会提示并退出。
# 可选: 设置 XDOTTER_TEST_NO_ISOLATION=1 时直接运行 test_xd.py（不隔离）

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

# 若明确要求不隔离，直接跑测试
if [ "${XDOTTER_TEST_NO_ISOLATION:-0}" = "1" ]; then
    exec python3 "$REPO_ROOT/test_xd.py" "$@"
fi

# 检查 bwrap
if ! command -v bwrap &>/dev/null; then
    echo "错误: 未找到 bwrap (bubblewrap)。" >&2
    echo "  在隔离环境中运行测试需要 bwrap。" >&2
    echo "  安装示例: sudo apt install bubblewrap  或  sudo dnf install bubblewrap" >&2
    echo "" >&2
    echo "  若仅想运行测试而不隔离，可执行:" >&2
    echo "    XDOTTER_TEST_NO_ISOLATION=1 python3 $REPO_ROOT/test_xd.py" >&2
    exit 1
fi

# 准备隔离用的假 HOME（主机临时目录，在容器内挂到固定路径避免与 /tmp 冲突）
FAKE_HOME_HOST="$(mktemp -d)"
mkdir -p "$FAKE_HOME_HOST/.cache"
cleanup() { rm -rf "$FAKE_HOME_HOST"; }
trap cleanup EXIT

# 容器内使用固定路径，避免 --tmpfs /tmp 覆盖导致路径不可见
FAKE_HOME_GUEST="/home/xdtest"

echo "=============================================="
echo "  隔离环境运行测试 (bwrap)"
echo "=============================================="
echo "  假 HOME (容器内): $FAKE_HOME_GUEST"
echo "  仓库:             $REPO_ROOT"
echo "=============================================="
echo ""

bwrap \
  --ro-bind "$REPO_ROOT" /app \
  --bind "$FAKE_HOME_HOST" "$FAKE_HOME_GUEST" \
  --setenv HOME "$FAKE_HOME_GUEST" \
  --tmpfs /tmp \
  --ro-bind /usr /usr \
  --ro-bind /etc /etc \
  --ro-bind /lib /lib \
  --ro-bind /lib64 /lib64 \
  --chdir /app \
  python3 test_xd.py "$@"
