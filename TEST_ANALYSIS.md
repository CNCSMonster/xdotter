# xdotter 测试分析与 CI 方案

## 测试总览: **102 个测试全部通过** ✅

| 层级 | 测试数 | 状态 | 运行方式 |
|------|--------|------|----------|
| **单元测试** | 43 | ✅ 全部通过 | `cargo test` |
| **E2E 测试** | 59 | ✅ 全部通过 | `bash scripts/test-rust.sh` |
| **总计** | **102** | **✅ 全部通过** | — |

---

## 一、单元测试分布 (43 个)

### 按模块

| 模块 | 测试数 | 覆盖内容 |
|------|--------|---------|
| `permissions.rs` | 20 | 权限匹配、SSH/GPG 路径识别、glob 匹配、敏感文件模式 (id_rsa, *.pem, *.key, *.gpg, ~/.bashrc, ~/.zshrc, ~/.Xauthority 等) |
| `symlink.rs` | 8 | 路径冲突、循环检测、环形引用检测 |
| `main.rs` | 6 | `expand_path` (~ 展开、绝对/相对路径、Unicode) |
| `config.rs` | 6 | TOML/JSON 解析、空配置、格式检测 |
| `commands/mod.rs` | 3 | 依赖循环检测、symlink 源文件拒绝 |

### 完整测试列表

```
commands::tests::test_deploy_detects_cycle
commands::tests::test_deploy_no_cycle_linear
commands::tests::test_deploy_rejects_symlink_source

config::tests::test_detect_format
config::tests::test_parse_empty_toml
config::tests::test_parse_invalid_toml
config::tests::test_parse_links_only_toml
config::tests::test_parse_valid_toml
config::tests::test_validate_empty_toml

permissions::tests::test_get_required_permission_aws_credentials
permissions::tests::test_get_required_permission_git_credentials
permissions::tests::test_get_required_permission_gnupg
permissions::tests::test_get_required_permission_named_ssh_key
permissions::tests::test_get_required_permission_not_sensitive
permissions::tests::test_get_required_permission_pattern_id_rsa
permissions::tests::test_get_required_permission_pattern_key
permissions::tests::test_get_required_permission_pattern_pem
permissions::tests::test_get_required_permission_shell_config
permissions::tests::test_get_required_permission_ssh_authorized_keys
permissions::tests::test_get_required_permission_ssh_ed25519
permissions::tests::test_get_required_permission_ssh_rsa
permissions::tests::test_get_required_permission_tilde_path
permissions::tests::test_get_required_permission_token_file
permissions::tests::test_get_required_permission_xauthority
permissions::tests::test_glob_match_exact
permissions::tests::test_glob_match_id_rsa_prefix
permissions::tests::test_glob_match_pem_suffix
permissions::tests::test_glob_match_prefix
permissions::tests::test_glob_match_suffix

symlink::tests::test_detect_circular_direct_parent
symlink::tests::test_detect_circular_scenario
symlink::tests::test_no_circular_when_not_symlink
symlink::tests::test_no_loop_simple
symlink::tests::test_no_loop_through_symlink
symlink::tests::test_paths_would_conflict_no_conflict
symlink::tests::test_paths_would_conflict_parent_child
symlink::tests::test_paths_would_conflict_same_path

tests::test_expand_path_absolute
tests::test_expand_path_no_home
tests::test_expand_path_relative
tests::test_expand_path_tilde
tests::test_expand_path_tilde_only
tests::test_expand_path_unicode_home
```

---

## 二、E2E 测试覆盖 (59 个)

| 类别 | 测试数 | 覆盖的功能 |
|------|--------|-----------|
| 基础命令 | 3 | help, version, new |
| 配置解析 | 1 | TOML links + dependencies |
| Deploy 核心 | 5 | basic, dry-run, tilde, multiple, already-exists |
| Undeploy | 2 | undeploy, nonexistent-link |
| 标志位 | 3 | quiet, verbose, force |
| 交互式模式 | 2 | interactive no/yes |
| 错误处理 | 6 | nonexistent source/config, invalid TOML, empty config, unicode, absolute path |
| 配置格式 | 3 | comments, whitespace, single quotes |
| 权限检查 | 6 | check SSH key, fix SSH key, dry-run, pattern matching, deploy-integrated, multiple sensitive files |
| Validate 命令 | 7 | valid/invalid TOML/JSON, default files, multiple files, nonexistent file |
| Completion 命令 | 5 | bash, zsh, fish, no shell, invalid shell |
| Deploy 自动验证 | 3 | auto-validation invalid, no-validate flag, auto-validation valid |
| Symlink 安全 | 6 | loop detection, circular scenario, parent symlink fix, loop warning, circular detailed, parent fix interactive |
| 其他 | 7 | dependencies subdir, content verification, empty links, new no-overwrite, status deployed, status broken, status verbose |

---

## 三、CI 配置

### GitHub Actions Workflow (`.github/workflows/ci.yml`)

#### 触发条件
- `push` 到 `main` 分支或 `v*` 标签
- `pull_request` 到 `main` 分支
- `workflow_dispatch` 手动触发

#### Job 矩阵

| Job | 平台 | 触发条件 | 失败阻塞 | 说明 |
|-----|------|----------|----------|------|
| `rust-check` | Linux/macOS/Windows | 每次 push/PR | ✅ | 编译检查 |
| `rust-clippy` | Linux/macOS | 每次 push/PR | ✅ | 代码质量 |
| `rust-fmt` | Linux/macOS | 每次 push/PR | ✅ | 格式化 |
| `rust-test` | Linux/macOS/Windows | 每次 push/PR | ✅ | 单元测试 |
| `rust-e2e` | Linux/macOS | 每次 push/PR | ✅ | E2E 集成测试 |
| `rust-binary-size` | Linux/macOS | 仅 release tag | ⚠️ | 二进制大小 ≤ 1MB |
| `rust-build` | Windows | 每次 push/PR | ✅ | Release 构建 |
| `release-*` | Linux/macOS/Windows | 仅 release tag | - | Release 二进制上传 |
| `upload-release` | Ubuntu | 仅 release tag | - | 创建 GitHub Release |

#### 平台覆盖对比

| 检查项 | Linux | macOS | Windows |
|--------|-------|-------|---------|
| cargo check | ✅ | ✅ | ✅ |
| cargo clippy | ✅ | ✅ | - |
| cargo fmt --check | ✅ | ✅ | - |
| cargo test | ✅ | ✅ | ✅ |
| E2E 测试 | ✅ | ✅ | - |
| Binary size | ✅ | ✅ | - |
| Release 构建 | ✅ | ✅ | ✅ |

---

## 四、技术细节

### 测试隔离机制
- **单元测试**：使用 `serial_test` crate 避免全局状态竞争（HOME、当前目录）
- **E2E 测试**：每个测试独立临时目录，测试结束自动清理
- **跨平台路径**：使用 `canonicalize()` 处理 macOS `/tmp → /private/tmp` 差异

### 关键测试设计

**依赖循环检测** (`commands/mod.rs`)：
```rust
#[test]
#[serial]  // 避免全局环境竞争
fn test_deploy_detects_cycle() {
    // a → b → a 循环检测
}
```

**E2E 测试脚本** (`scripts/test-rust.sh`)：
```bash
run_xd() {
    local home_dir="$1"; shift
    (cd "$home_dir" && HOME="$home_dir" "$RUST_BIN" "$@" 2>&1)
}
```

### 运行测试

```bash
# 单元测试
cargo test

# E2E 集成测试
cargo build && bash scripts/test-rust.sh

# 全部测试（推荐提交前运行）
cargo test && cargo build && bash scripts/test-rust.sh
```
