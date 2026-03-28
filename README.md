# xdotter: a simple dotfile manager

A **zero-dependency**, **single-file** dotfile manager written in Python. No build tools, no package managers—just download and run.

**Now with robust TOML parsing powered by [tomli](https://github.com/hukkin/tomli)!**

## Features

- ✅ **Zero dependencies** - Uses Python standard library + vendored tomli
- ✅ **Single file** - Easy to distribute and understand (~55KB .pyz)
- ✅ **Cross-platform** - Works on Linux, macOS, and Windows (with Python)
- ✅ **No installation required** - Run directly or install with one command
- ✅ **Fast & Simple** - Minimal overhead, easy to configure
- ✅ **Robust TOML parsing** - Full TOML v1.0 compliance via embedded tomli
- ✅ **Permission checking** - Auto-detect and fix permissions for sensitive files

## Quick Start

```bash
# Download (auto-detects authenticated gh, falls back to curl)
if command -v gh &> /dev/null && gh auth status &> /dev/null 2>&1; then
    gh release download --repo cncsmonster/xdotter --pattern xd.pyz --output ~/.local/bin/xd
else
    curl -L https://github.com/cncsmonster/xdotter/releases/latest/download/xd.pyz -o ~/.local/bin/xd
fi

# Make executable
chmod +x ~/.local/bin/xd

# Run
xd --help
```

**Note:** Using `gh` avoids GitHub rate limits (5000/hour vs 60/hour for unauthenticated requests).

## Development

```bash
git clone https://github.com/cncsmonster/xdotter.git
cd xdotter
python3 xd.py --help
```

## Usage

```bash
# Show help
xd --help

# Create a new configuration template
xd new

# Deploy dotfiles
xd deploy

# Deploy with verbose output
xd deploy -v

# Dry-run (see what would happen)
xd deploy -n

# Undeploy (remove symlinks)
xd undeploy

# Undeploy with confirmation
xd undeploy -i
```

### Commands

| Command | Description |
|---------|-------------|
| `deploy` | Deploy dotfiles (default) |
| `undeploy` | Remove deployed dotfiles |
| `check-permissions` | Check/fix permissions for deployed files |
| `validate` | Validate configuration file syntax |
| `new` | Create a new `xdotter.toml` template |
| `help` | Print help message |
| `version` | Print version |

### Options

| Option | Description |
|--------|-------------|
| `-v, --verbose` | Show more information |
| `-q, --quiet` | Do not print any output |
| `-n, --dry-run` | Show what would be done without making changes |
| `-i, --interactive` | Ask for confirmation when unsure |
| `-f, --force` | Force overwrite existing files |
| `--check-permissions` | Check permissions for sensitive files (SSH, GPG, etc.) |
| `--fix-permissions` | Fix permissions for sensitive files |
| `--no-validate` | Skip config syntax validation during deploy |

### Permission Checking

xdotter can check and fix permissions for sensitive files based on their target location:

```bash
# Check permissions during deployment
xd deploy --check-permissions

# Check and fix permissions during deployment
xd deploy --fix-permissions

# Dry-run to see what would be fixed
xd deploy --fix-permissions -n

# Check permissions for already deployed files
xd check-permissions

# Fix permissions for deployed files
xd check-permissions --fix-permissions
```

### Configuration Validation

xdotter automatically validates configuration syntax before deployment:

```bash
# Validate configuration (auto-run during deploy)
xd validate

# Validate specific files
xd validate myconfig.toml
xd validate config1.toml config2.json

# Skip validation during deploy (emergency)
xd deploy --no-validate
```

**Supported formats:**

| Format | Extension | Validator |
|--------|-----------|-----------|
| TOML | `.toml` | tomli (vendored) |
| JSON | `.json` | json (stdlib) |

**Error output includes:**
- Line and column numbers
- Context (surrounding lines)
- Fix suggestions in Chinese

**Supported sensitive paths:**

| Path | Required Permission | Description |
|------|---------------------|-------------|
| `~/.ssh/` | 700 | SSH directory |
| `~/.ssh/id_rsa`, `id_ed25519`, etc. | 600 | SSH private keys |
| `~/.ssh/authorized_keys` | 600 | SSH authorized keys |
| `~/.gnupg/` | 700 | GPG directory |
| `~/.gnupg/private-keys-v1.d/` | 700 | GPG private keys directory |
| `~/.netrc` | 600 | Netrc password file |
| `~/.pgpass` | 600 | PostgreSQL password file |
| `~/.bashrc`, `~/.zshrc` | 644 | Shell configs (affect PATH and env vars) |
| `~/.bash_profile`, `~/.profile` | 644 | Login profiles |
| `~/.xinitrc`, `~/.xsession` | 755 | X11 session scripts (must be executable) |
| `~/.xprofile` | 644 | X session environment |
| `~/.Xauthority` | 600 | X11 authentication |

**Filename patterns:**

Files matching these patterns are automatically detected as sensitive:

- `id_rsa*`, `id_ed25519*`, `id_ecdsa*`, `id_dsa*` - SSH private keys
- `*_rsa`, `*_ed25519`, `*_ecdsa`, `*_dsa` - Named SSH private keys
- `*.pem`, `*.key` - Certificate/key files
- `*.gpg`, `*.asc` - GPG files
- `*.bashrc`, `*.zshrc`, `*.profile` - Shell config backups

## Configuration

Create an `xdotter.toml` file:

```toml
# xdotter configuration file

[links]
# Format: "source_path" = "target_link"
# The source is your actual dotfile in the repo
# The target is where you want it symlinked (~ expands to home directory)

".config/nvim/init.lua" = "~/.config/nvim/init.lua"
".zshrc" = "~/.zshrc"
".gitconfig" = "~/.gitconfig"

[dependencies]
# Format: "name" = "relative_path"
# Subdirectories with their own xdotter.toml
# "go" = "testdata/go"
# "nvim" = "config/nvim"
```

## Example Workflow

```bash
# 1. Create a new config template
xd new

# 2. Edit xdotter.toml with your dotfiles

# 3. Deploy everything
xd deploy

# 4. Later, undeploy if needed
xd undeploy
```

## Why Python?

The previous Rust version required:
- ❌ Rust toolchain installation
- ❌ Compilation time
- ❌ `cargo install` or building from source

This Python version:
- ✅ Works wherever Python 3 exists (pre-installed on most systems)
- ✅ No compilation needed
- ✅ Single file (.pyz), easy to distribute
- ✅ Download and use immediately
- ✅ Robust TOML parsing with embedded tomli

## Requirements

- Python 3.8+ (required by vendored tomli)
- Unix-like system (Linux, macOS) or Windows with Python

**Note:** Python 3.11+ has a built-in `tomllib`; this project uses **vendored [tomli](https://github.com/hukkin/tomli)** so it works on **Python 3.8, 3.9, 3.10** without any standard-library TOML. CI runs on 3.8, 3.10, and 3.12 to verify.

## What about the .pyz file?

The `.pyz` file is a **single-file executable Python archive** (PEP 441). It:
- ✅ Contains all code and dependencies (including tomli)
- ✅ Runs with any Python 3.8+ interpreter
- ✅ Is completely transparent (it's just a zip file)
- ✅ Can be inspected with `unzip -l xd.pyz`
- ✅ Works exactly like a `.py` file: `python3 xd.pyz deploy`

**Why .pyz instead of .py?**
- Building `.pyz` is **1 command**: `python -m zipapp ...`
- Manually merging code is **very complex** (import handling, namespaces, etc.)
- `.pyz` is **industry standard** (used by pip, shiv, etc.)
- User experience is **identical**: download → run

## Testing

Run the test suite to verify all functionality:

```bash
python3 test_xd.py
```

**Python 3.8–3.12** compatibility (including 3.8/3.10 without standard-library `tomllib`) is verified in **CI** on every push; see [.github/workflows/ci.yml](.github/workflows/ci.yml).

**Test Coverage (37 tests):**

| Category | Tests |
|----------|-------|
| CLI Commands | help, version, new |
| Config Parsing | sections, comments, whitespace, quotes |
| Deploy | basic link, dry-run, tilde expansion, multiple links |
| Undeploy | remove symlink, nonexistent link |
| Flags | quiet, verbose, force |
| Interactive | confirm yes/no |
| Edge Cases | nonexistent source/config, invalid TOML, empty config |
| Special Cases | unicode paths, absolute paths, existing symlink |
| Dependencies | subdirectory deployment |
| Permission Check | SSH key detection, fix, correct permission, pattern matching, dry-run |

## Container Testing

Test with isolated environment using bubblewrap:

```bash
./scripts/bwrap-test.sh
```

This runs a complete deployment test of `cncsmonster/dotfiles` in an isolated sandbox.

## Building

```bash
./scripts/build-pyz.sh
```

## Shell Completion

xdotter uses [argcomplete](https://github.com/kislyuk/argcomplete) for automatic shell completion generation from argparse definitions.

### Bash

```bash
# One-time setup
eval "$(register-python-argcomplete xd)"

# Or add to ~/.bashrc for permanent:
echo 'eval "$(register-python-argcomplete xd)"' >> ~/.bashrc

# If xd is installed via .pyz, use:
eval "$(register-python-argcomplete --external-argcomplete-script ~/.local/bin/xd xd)"
```

### Zsh

```bash
# One-time setup
eval "$(register-python-argcomplete --shell zsh xd)"

# Or add to ~/.zshrc for permanent:
echo 'eval "$(register-python-argcomplete --shell zsh xd)"' >> ~/.zshrc
```

### Fish

```bash
# One-time setup
register-python-argcomplete --shell fish xd | source

# Or add to config.fish for permanent:
register-python-argcomplete --shell fish xd >> ~/.config/fish/completions/xd.fish
```

### Alternative: Static Completion Scripts

If you prefer static completion scripts (no argcomplete dependency):

```bash
# Bash
xd completion bash > ~/.local/share/bash-completion/completions/xd

# Zsh
xd completion zsh > ~/.local/share/zsh/site-functions/_xd

# Fish
xd completion fish > ~/.config/fish/completions/xd.fish
```

## License

[MIT License](LICENSE)

## Contributing

Contributions are welcome! Feel free to open an issue or submit a PR.
