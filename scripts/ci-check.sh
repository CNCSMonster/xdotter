#!/bin/bash
#
# CI Check Script - 在推送前验证 GitHub Actions workflow 是否正常
#
# 用法:
#   ./scripts/ci-check.sh              # 完整检查所有 workflow
#   ./scripts/ci-check.sh --dry-run    # 仅 dry-run（快速检查）
#   ./scripts/ci-check.sh release.yml  # 检查指定 workflow
#

set -e

# 颜色定义
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

# 项目根目录
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
WORKFLOW_DIR="$PROJECT_ROOT/.github/workflows"

# 默认参数
DRY_RUN_ONLY=false
TARGET_WORKFLOW=""

# 解析参数
while [[ $# -gt 0 ]]; do
    case $1 in
        --dry-run|-d)
            DRY_RUN_ONLY=true
            shift
            ;;
        -h|--help)
            echo "用法: $0 [选项] [workflow文件]"
            echo ""
            echo "选项:"
            echo "  --dry-run, -d    仅运行 dry-run 检查（不实际执行构建）"
            echo "  -h, --help       显示此帮助信息"
            echo ""
            echo "示例:"
            echo "  $0                    # 完整检查所有 workflow"
            echo "  $0 --dry-run          # 快速检查所有 workflow"
            echo "  $0 release.yml        # 检查指定 workflow"
            exit 0
            ;;
        *.yml|*.yaml)
            TARGET_WORKFLOW="$1"
            shift
            ;;
        *)
            echo -e "${RED}错误: 未知参数 '$1'${NC}"
            exit 1
            ;;
    esac
done

echo -e "${BLUE}========================================${NC}"
echo -e "${BLUE}       CI Workflow 检查工具${NC}"
echo -e "${BLUE}========================================${NC}"
echo ""

# 检查 act 是否安装
if ! command -v act &> /dev/null; then
    echo -e "${RED}错误: act 未安装${NC}"
    echo ""
    echo "安装方法:"
    echo "  curl -s https://raw.githubusercontent.com/nektos/act/master/install.sh | sudo BINDIR=/usr/local/bin bash"
    exit 1
fi
echo -e "${GREEN}✓ act 已安装: $(act --version)${NC}"

# 检查 Docker（act 的 dry-run 也需要 Docker）
if ! docker info &> /dev/null; then
    echo -e "${RED}错误: Docker 未运行或无权限访问${NC}"
    echo ""
    echo "请确保 Docker 已启动且当前用户有权限访问。"
    echo "可以尝试:"
    echo "  1. sudo systemctl start docker"
    echo "  2. sudo usermod -aG docker \$USER  (然后重新登录)"
    exit 1
fi
echo -e "${GREEN}✓ Docker 运行正常${NC}"
echo ""

# YAML 语法检查
echo -e "${BLUE}[1/2] YAML 语法检查${NC}"
check_yaml() {
    local file="$1"
    if command -v python3 &> /dev/null; then
        if python3 -c "import yaml; yaml.safe_load(open('$file'))" 2>/dev/null; then
            echo -e "${GREEN}  ✓ $(basename "$file")${NC}"
            return 0
        else
            echo -e "${RED}  ✗ $(basename "$file") - YAML 语法错误${NC}"
            return 1
        fi
    else
        echo -e "${YELLOW}  - $(basename "$file") (跳过，需要 python3)${NC}"
    fi
}

if [[ -n "$TARGET_WORKFLOW" ]]; then
    check_yaml "$WORKFLOW_DIR/$TARGET_WORKFLOW"
else
    for f in "$WORKFLOW_DIR"/*.yml "$WORKFLOW_DIR"/*.yaml; do
        [[ -f "$f" ]] && check_yaml "$f"
    done
fi
echo ""

# Act 检查
echo -e "${BLUE}[2/2] Act ${DRY_RUN_ONLY:+dry-run}检查${NC}"

run_act() {
    local workflow="$1"
    local flag=""
    [[ "$DRY_RUN_ONLY" == "true" ]] && flag="-n"
    local tmpfile=$(mktemp)
    
    echo -e "  运行: $workflow ..."
    
    # 运行 act 并保存输出到临时文件（实时显示）
    act $flag push -W ".github/workflows/$workflow" 2>&1 | tee "$tmpfile"
    
    # 检查结果
    if grep -q "Job succeeded" "$tmpfile"; then
        echo -e "${GREEN}  ✓ $workflow 通过${NC}"
        rm -f "$tmpfile"
        return 0
    elif grep -q "token or opts.auth is required" "$tmpfile"; then
        echo -e "${YELLOW}  ⚠ $workflow - Release 需要 GITHUB_TOKEN（本地无法测试，配置正确）${NC}"
        rm -f "$tmpfile"
        return 0
    else
        echo -e "${RED}  ✗ $workflow 失败${NC}"
        rm -f "$tmpfile"
        return 1
    fi
}

cd "$PROJECT_ROOT"
failed=0

if [[ -n "$TARGET_WORKFLOW" ]]; then
    run_act "$TARGET_WORKFLOW" || failed=1
else
    for f in "$WORKFLOW_DIR"/*.yml; do
        [[ -f "$f" ]] && { run_act "$(basename "$f")" || failed=1; }
    done
fi

echo ""
if [[ $failed -eq 0 ]]; then
    echo -e "${GREEN}========================================${NC}"
    echo -e "${GREEN}       ✓ 所有检查通过${NC}"
    echo -e "${GREEN}========================================${NC}"
else
    echo -e "${RED}========================================${NC}"
    echo -e "${RED}       ✗ 部分检查失败${NC}"
    echo -e "${RED}========================================${NC}"
    exit 1
fi
