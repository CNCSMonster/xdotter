# 项目冗余文件分析

> 分析时间：基于当前仓库文件列表与 CI/workflow 使用情况。

## 结论概览

| 类型 | 建议 | 说明 |
|------|------|------|
| **docker-test.sh** | 可删或标为可选 | 与 bwrap-test.sh 功能重叠，CI 未用 |
| **xd.pyz** | 建议不提交 / 或保留 | 构建产物，通常 CI 构建即可 |
| **_vendor/.gitkeep** | 可删 | _vendor 已有内容，占位无必要 |
| **bwrap-test.sh** | 保留 | 与 run-tests-isolated 用途不同（集成 vs 单元） |
| **scripts/ci-check.sh** | 保留 | 本地验证 workflow，有用 |
| **scripts/run-tests-isolated.sh** | 保留 | CI 与本地隔离单元测试用 |

---

## 1. 根目录脚本

### bwrap-test.sh（保留）

- **用途**：用 bwrap 做「完整 dotfiles 集成测试」——克隆/使用 cncsmonster/dotfiles，在隔离 HOME 下跑 deploy / undeploy，并验证 symlink、写日志。
- **与 run-tests-isolated.sh 区别**：
  - `run-tests-isolated.sh`：只跑 **test_xd.py**（单元测试），假 HOME 下不碰真实 dotfiles。
  - `bwrap-test.sh`：用**真实 dotfiles 仓库**在隔离环境里做端到端部署测试。
- **结论**：功能不重复，保留。建议 README 中区分「单元测试隔离」与「dotfiles 集成测试」。

### docker-test.sh（冗余 / 可选）

- **用途**：用 Docker + `python:3.11-slim` 跑与 bwrap-test.sh 类似的「dotfiles 部署集成测试」（克隆 dotfiles、deploy、undeploy、写日志与 test-report.md）。
- **与 bwrap-test.sh**：目标一致，实现不同（Docker vs bwrap）。CI 未使用二者。
- **结论**：与 bwrap-test.sh 功能重叠；bwrap 无需 Docker、更轻量。可删除 docker-test.sh，或保留并注明「可选/备用」。

---

## 2. 构建产物

### xd.pyz

- **用途**：单文件 zipapp 分发包。
- **当前**：被 git 跟踪并提交。
- **常见做法**：在 CI 中构建，通过 GitHub Releases 发布，不提交到仓库，避免仓库体积和与 xd.py/_vendor 不同步。
- **结论**：可视为「可清理的冗余」——改为 CI 构建 + 不提交；若希望 clone 即用可保留，但建议至少在文档中说明「推荐从 Releases 下载」。

---

## 3. _vendor

### _vendor/.gitkeep

- **用途**：通常用于让 git 保留空目录。
- **当前**：_vendor 下已有 README.md、__init__.py、tomli/ 等，目录非空。
- **结论**：占位已无必要，可删除，属轻微冗余。

---

## 4. 脚本与 CI

### scripts/ci-check.sh（保留）

- **用途**：本地用 act 模拟运行 GitHub Actions workflow，检查 YAML 语法、可 dry-run。依赖 act + Docker。
- **CI**：未被 workflow 调用，仅用于推送前本地验证。
- **结论**：非冗余，保留有价值。

### scripts/run-tests-isolated.sh（保留）

- **用途**：在 bwrap 隔离的假 HOME 下运行 test_xd.py，CI 已使用。
- **结论**：核心脚本，保留。

---

## 5. 未跟踪 / 已忽略

- **.claude/settings.local.json**：若未加入 git，则无需处理；个人/编辑器配置一般不必提交。
- **__pycache__/**、**_vendor/tomli/__pycache__/**：已在 .gitignore，只要未误提交即可。

---

## 建议执行操作（可选）

1. **删除 docker-test.sh**（若不需要 Docker 版集成测试）。
2. **删除 _vendor/.gitkeep**。
3. **xd.pyz**：二选一  
   - 从仓库删除并在 .gitignore 中加入 `xd.pyz`，在 CI 中构建并仅通过 Release 发布；或  
   - 保留提交，但在 README 中说明「推荐从 Releases 下载」。
4. **README**：在「Container Testing」一节区分：
   - 单元测试隔离：`./scripts/run-tests-isolated.sh`
   - Dotfiles 集成测试：`./bwrap-test.sh`
