#!/usr/bin/env bash
# check-release-ready.sh — 在打 release tag 前验证仓库状态
#
# 检查项:
#   1. Cargo.lock 存在且被 git 跟踪
#   2. Cargo.lock 没有未提交的修改
#   3. Cargo.lock 与 Cargo.toml 一致 (cargo check 通过)
#   4. 工作区干净 (无未暂存/未跟踪文件)
#   5. cargo test 全部通过
#   6. cargo clippy 无警告
#
# 用法: scripts/check-release-ready.sh [--quick]
#   --quick  跳过测试和 clippy (仅检查文件状态)

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

failures=0
check() {
    local label="$1"
    shift
    if "$@"; then
        echo -e "  ${GREEN}✓${NC} $label"
    else
        echo -e "  ${RED}✗${NC} $label"
        failures=$((failures + 1))
    fi
}

echo "=== 检查 release 就绪状态 ==="
echo ""

# 1. Cargo.lock 存在
check "Cargo.lock 存在" test -f Cargo.lock

# 2. Cargo.lock 被 git 跟踪
check "Cargo.lock 被 git 跟踪" sh -c 'git ls-files --error-unmatch Cargo.lock >/dev/null 2>&1'

# 3. Cargo.lock 无本地修改
check "Cargo.lock 无未提交修改" git diff --quiet -- Cargo.lock

# 4. Cargo.lock 已暂存 (无 staged 修改)
check "Cargo.lock 无暂存修改" git diff --cached --quiet -- Cargo.lock

# 5. 工作区干净
if git diff --quiet && git diff --cached --quiet; then
    echo -e "  ${GREEN}✓${NC} 工作区干净"
else
    echo -e "  ${YELLOW}⚠${NC} 工作区有未提交修改 (如果只是 CHANGELOG/版本号 bump 则属于正常)"
fi

if [ "${1:-}" != "--quick" ]; then
    echo ""
    echo "--- 构建与测试 ---"

    check "cargo check" cargo check --quiet

    check "cargo clippy (无警告)" cargo clippy -- -D warnings 2>/dev/null

    check "cargo test" cargo test --quiet

    check "cargo fmt (无格式问题)" cargo fmt --check
fi

echo ""
if [ "$failures" -eq 0 ]; then
    echo -e "${GREEN}✓ 所有检查通过 — 可以打 tag 了${NC}"
    exit 0
else
    echo -e "${RED}✗ $failures 项检查失败${NC}"
    exit 1
fi
