# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.5.0]

### Added
- **Project specification** — Added `SPEC.md` as the source of truth for xdotter behavior, safety semantics, testing requirements, and documentation rules.
- **Three-stage lifecycle** — `discover → plan → apply` modules separate config loading, planning, and apply work; planning aggregates errors across the root and all reachable dependencies before reporting.
- **Error classification labels** — Every error message now carries one of `[CLI 参数错误]` / `[配置错误]` / `[规划阻塞错误]` / `[应用阶段错误]` so users and scripts can classify failures.
- **Global link uniqueness check** — Two or more `[links]` entries that expand to the same link path are reported as a single configuration error that enumerates **every** offending `(config file, source path)` pair.
- **SPEC seven-line status summary** — `xd status` prints a fixed seven-line summary plus the `Status: N/M deployed` line; counters cover deployed / not-deployed / wrong / broken / source-missing / source-type-invalid / non-symlink / permission-issues.
- **Configuration directory tree boundary** — Each `xdotter.toml` is scoped to its own directory tree; source and dependency paths must remain inside it after canonicalization.

### Changed (BREAKING)
- **CLI argument model** — `--force`, `--interactive`, `--dry-run` are now command-scoped operation flags on `deploy` / `undeploy` (and `--dry-run` on `new`). `-v`/`--verbose` is the sole global diagnostic flag and is now repeatable (`-v`, `-vv`, `-vvv`). The previous global-flag layout (`xd -v deploy`) is no longer accepted.
- **`--force` and `--interactive` are mutually exclusive** at CLI parsing time.
- **`xd undeploy` default behavior** — Now only deletes correct or broken symlinks; wrong symlinks are recoverable conflicts requiring `--force` (auto-delete) or `--interactive` (per-link confirmation). Previously the default would delete any symlink at the link path.
- **Permission targets restricted to the SPEC table** — Only the eight target classes listed in SPEC §"权限和敏感文件语义" are checked: `~/.ssh`, `~/.ssh/config`, `~/.ssh/authorized_keys`, `~/.ssh/id_*` (non-`.pub`), `~/.ssh/*_{rsa,ed25519,ecdsa,dsa}` (non-`.pub`), `~/.pgpass`, `~/.netrc`, `~/.gnupg`. Shell-config / AWS / Docker / npm / `*.pem` / `*.key` / `*.token` and similar entries from earlier versions are no longer subject to built-in checks. Source-content sniffing of `.pub` files has been removed; classification is based purely on the expanded link path.
- **TOML parsing rejects unknown top-level keys/tables** as configuration errors.
- **Apply stage stops on first failure** rather than continuing through subsequent links. Partial progress is not rolled back.

### Removed (BREAKING)
- **`xd validate` subcommand** — Configuration validity is checked automatically; there is no separate validation command.
- **Global flags** `--quiet`, `--check-permissions`, `--fix-permissions`, `--no-validate` — removed. Permission handling is now part of `deploy`'s recoverable-conflict model and is governed by the active conflict mode (default skip / `--force` fix / `--interactive` ask).
- **JSON configuration support** — Only TOML remains supported.
- **Built-in parent-symlink auto-replacement** — Unsafe parent components are now planning-block errors; xdotter will no longer remove or replace any parent of a link path.

### Fixed (SPEC compliance audit)
- **Apply loop semantics** — Per SPEC §"按链接路径状态划分" + §"应用阶段错误", user rejection in interactive mode and "link path is not a symlink" in undeploy are recoverable skips that count as failures but **do not stop the loop**. Previously the first such case aborted all subsequent links. Apply-stage system errors (OS errors, state re-check failures) still stop the loop as required.
- **Sensitive-target warning** — Per SPEC §"权限和敏感文件语义", deploy now unconditionally emits a stderr warning whenever a link path matches a built-in permission target, regardless of whether the permission is already correct. Previously this warning was missing.
- **`--interactive --dry-run` rendering** — Per SPEC §"执行模式", the dry-run output now shows recoverable conflicts as "would skip (interactive declined)" rather than promising replacement. `--force --dry-run` continues to render as "replace".
- **Link-path ancestor topology checks** — Per SPEC §"符号链接安全语义", planning now walks **all** existing ancestors of the link path, not only the direct parent: any ancestor that is a regular file (or other non-directory) is a planning-block error, and unsafe symlinked ancestors are detected at any depth. Previously deeper-than-parent ancestors were only caught at apply time as `create_dir_all` failures.
- **CLI-error classification label** — clap's own diagnostics (e.g. mutex flag rejections, unknown subcommand) are now wrapped with the `[CLI 参数错误]` classification prefix so all error output satisfies SPEC §"输出语义" without exception.
- **Verbose output non-duplication** — Per SPEC §"输出语义", `-v` no longer reports `SkipFailure` and `NotASymlinkWarning` actions twice (once in verbose per-action summary and again in the error output). Each event appears exactly once.
- **Multi-error label preservation** — Per SPEC §"输出语义", when multiple errors are reported together (e.g. several skipped links in one deploy run), each error line now independently carries its own `[分类标签]`; the aggregation no longer wraps them under a single outer label that would misclassify lines with a different error class.
- **`-v/-vv/-vvv` verbosity** — All commands now honor the verbose level: `deploy`, `undeploy`, and `new` emit progress / per-link diagnostics on stderr at appropriate levels. Previously only `status` consulted the verbosity counter.

---

## [0.4.1] - 2026-04-13

### Added
- **Nightly build-std optimization for release** — CI now uses `cargo +nightly build -Z build-std` to produce ~392KB binaries (down from ~640KB)
- **Automatic GitHub Release upload** — Release binaries for Linux/macOS/Windows are automatically uploaded to GitHub Releases when a tag is pushed

### Changed
- **Updated CI workflow** — Split into stable (check/test) and nightly (release build) stages
- **Cleaned repository** — Removed all Python implementation files, vendored dependencies, and obsolete design docs

---

## [0.4.0] - 2026-04-12

### Rust Rewrite
- **Complete rewrite in Rust** — Single static binary, no runtime dependencies
- **Binary size: ~683KB** (release build with LTO + strip + panic=abort)
- **Dependencies**: clap (CLI), basic-toml (parsing), serde (derive), dirs (home detection), clap_complete (shell completion)
- **Cross-platform support** — Builds and runs on Linux, macOS, and Windows

### Added
- **`status` command** — Show deployment status (valid, broken, permission issues)
- **Shell completion generation** — Auto-generated via `clap_complete` at compile time for bash, zsh, fish
- **Enhanced symlink safety** — Loop detection, circular symlink detection, path conflict detection
- **Parent symlink auto-fix** — `--force` flag automatically fixes parent directory symlink issues
- **Config auto-validation** — Automatically validates TOML/JSON syntax before deployment
- **Permission checking during deploy** — `--check-permissions` and `--fix-permissions` flags
- **89 tests** — 39 unit tests + 50 E2E integration tests, all passing

### Changed
- **Permission checking as flags** — No longer a separate `check-permissions` command; now `--check-permissions` and `--fix-permissions` flags during deploy
- **Fixed config filename** — Always uses `xdotter.toml` or `xdotter.json` in current directory (removed `-c`/`--config` parameter)
- **Improved error messages** — Color-coded output (yellow warnings, red errors), Chinese language hints for TOML errors
- **Flags before subcommand** — All global flags (`-v`, `-q`, `-n`, `-f`, `-i`) must appear before the command: `xd -v deploy` not `xd deploy -v`

### Removed
- **`check-permissions` subcommand** — Replaced by `--check-permissions` and `--fix-permissions` flags
- **Python runtime dependency** — Now a single static Rust binary

---

## [0.3.4] - 2026-04-03

### Fixed
- **`--force` flag now auto-fixes parent directory symlink issues** - When the target's parent directory is a symlink, `--force` will automatically remove the parent symlink and create a real directory
  - Previously required `-i` (interactive mode) to fix this scenario
  - Now `--force` handles it automatically, consistent with the "force" semantics
  - Without `--force` or `-i`, deployment is skipped with a warning

### Added
- **Automatic shell completion via argcomplete** - Shell completion now auto-generated from argparse
  - Vendored argcomplete 3.6.3 (~42KB, no runtime dependencies)
  - Completion automatically stays in sync with CLI definition
  - Use `xd completion <bash|zsh|fish>` to generate completion scripts
  - Supports Bash, Zsh, and Fish out of the box
  - No external tools required (e.g., `register-python-argcomplete`)
  - No manual maintenance of completion scripts required

- **Shell completion support** - Generate completion scripts for Bash, Zsh, and Fish
  - `xd completion bash` - Generate Bash completion script
  - `xd completion zsh` - Generate Zsh completion script
  - `xd completion fish` - Generate Fish completion script
  - Installation instructions in README.md

- **New `validate` command** - Check configuration file syntax before deployment
  - `xd validate` - Validate xdotter.toml or xdotter.json
  - `xd validate file1.toml file2.json` - Validate specific files
  - Supports both TOML and JSON formats
  - Provides detailed error messages with line numbers and suggestions

- **Auto-validation during deploy** - Configuration syntax is automatically checked before deployment
  - Prevents deployment with invalid configurations
  - Use `--no-validate` to skip validation (emergency situations)

- **TOML error suggestions** - Common TOML errors include fix suggestions:
  - Unclosed strings, missing `=`, invalid keys, etc.
  - Chinese language hints for common issues

- **JSON error suggestions** - Common JSON errors include fix suggestions:
  - Missing commas, invalid escapes, unterminated strings, etc.
  - Chinese language hints for common issues

### Changed
- **`--force` flag now auto-fixes parent directory symlink issues** - When the target's parent directory is a symlink, `--force` will automatically remove the parent symlink and create a real directory
  - Previously required `-i` (interactive mode) to fix this scenario
  - Now `--force` handles it automatically, consistent with the "force" semantics
  - Without `--force` or `-i`, deployment is skipped with a warning

- **Removed `-c`/`--config` parameter** - Configuration file is now fixed to `xdotter.toml` in the current directory
  - This simplifies the CLI and aligns with the "simple by default" philosophy
  - If you need different configurations, use different directories or rename the config file

- **New `check-permissions` command** - Check and fix permissions for already deployed files
  - `xd check-permissions` - Check permissions for all deployed symlinks
  - `xd check-permissions --fix-permissions` - Check and automatically fix permissions
  - `xd check-permissions --fix-permissions -n` - Dry-run to preview fixes
  
- **Shell configuration permission checks** - Added permission checks for shell configs:
  - `~/.bashrc`, `~/.zshrc` (644)
  - `~/.bash_profile`, `~/.profile`, `~/.zprofile` (644)
  - `~/.zshenv`, `~/.zlogin`, `~/.bash_logout` (644)
  
- **X11/GUI permission checks** - Added permission checks for GUI-related files:
  - `~/.xinitrc`, `~/.xsession` (755 - must be executable)
  - `~/.xprofile` (644)
  - `~/.Xauthority` (600 - contains authentication data)
  - `~/.Xresources`, `~/.Xdefaults` (644)

- **Permission check behavior during deployment**:
  - When using `--check-permissions` or `--fix-permissions`, files with incorrect permissions will trigger a warning
  - Deployment is **skipped** for files with permission issues (unless `--force` is used)
  - `--fix-permissions` automatically fixes permissions before deployment

### Fixed
- Permission check now validates **source file** permissions (not target path)
- Permission pattern matching for SSH keys and other sensitive files

---

## [0.3.3] - 2024-03-28

### Fixed
- Permission pattern matching test cases

### Changed
- Improved test isolation with bubblewrap

---

## [0.3.2] - 2024-03-27

### Added
- Permission checking for sensitive files (SSH, GPG, etc.)
- `--check-permissions` flag to detect permission issues
- `--fix-permissions` flag to automatically fix permissions

### Changed
- Color-coded output: yellow for warnings, red for errors

---

## [0.3.1] - 2024-03-26

### Fixed
- Python 3.8 compatibility

---

## [0.3.0] - 2024-03-25

### Added
- `check-permissions` command for checking deployed files
- Support for filename pattern matching in permission checks

### Changed
- Improved permission check output and error messages

---

## [0.2.1] - 2024-03-24

### Fixed
- Various bug fixes and improvements

---

## [0.2.0] - 2024-03-23

### Added
- Interactive mode with `-i` flag
- Force mode with `-f` flag

---

## [0.1.0] - 2024-03-22

### Added
- Initial release
- Basic deploy/undeploy functionality
- TOML configuration support
