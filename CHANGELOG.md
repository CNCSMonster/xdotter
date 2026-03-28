# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
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
