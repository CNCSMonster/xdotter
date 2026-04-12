# xdotter: a simple dotfile manager

A **zero-dependency**, **single-binary** dotfile manager written in Rust. No runtime dependencies, no package managers—just download and run.

**Binary size: ~683KB** (release, optimized with LTO and stripping)

## Features

- ✅ **Zero runtime dependencies** - Single static binary, no external libraries needed
- ✅ **Single binary** - Easy to distribute (~683KB optimized release build)
- ✅ **Cross-platform** - Works on Linux and macOS (Windows support planned)
- ✅ **No installation required** - Download and run immediately
- ✅ **Fast & Simple** - Minimal overhead, easy to configure
- ✅ **Robust TOML parsing** - Full TOML v1.0 compliance via `basic-toml`
- ✅ **Permission checking** - Auto-detect and fix permissions for sensitive files
- ✅ **Symlink safety** - Loop detection, circular symlink detection, conflict resolution
- ✅ **Shell completion** - Auto-generated completion for bash, zsh, fish

## Quick Start

```bash
# Download (auto-detects authenticated gh, falls back to curl)
if command -v gh &> /dev/null && gh auth status &> /dev/null 2>&1; then
    gh release download --repo cncsmonster/xdotter --pattern 'xd' --output ~/.local/bin/xd
else
    curl -L https://github.com/cncsmonster/xdotter/releases/latest/download/xd -o ~/.local/bin/xd
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
cargo build --release
./target/release/xd --help
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
| `status` | Show deployment status |
| `validate` | Validate configuration file syntax |
| `new` | Create a new `xdotter.toml` template |
| `completion` | Generate shell completion scripts |
| `version` | Print version |

### Options

| Option | Description |
|--------|-------------|
| `-v, --verbose` | Show more information |
| `-q, --quiet` | Do not print any output |
| `-n, --dry-run` | Show what would be done without making changes |
| `-i, --interactive` | Ask for confirmation when unsure |
| `-f, --force` | Force overwrite existing files |
| `--check-permissions` | Check permissions for sensitive files during deploy |
| `--fix-permissions` | Fix permissions for sensitive files during deploy |
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
```

**Note:** Permission checking is only available as flags during the `deploy` command. It checks source file permissions before creating symlinks.

### Configuration Validation

xdotter automatically validates configuration syntax before deployment:

```bash
# Validate configuration (auto-run during deploy)
xd validate

# Validate specific files
xd validate myconfig.toml
xd validate config1.toml config2.toml

# Skip validation during deploy (emergency)
xd deploy --no-validate
```

**Supported format:** TOML only (`.toml`)

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

## Why Rust?

The Rust rewrite provides:
- ✅ **Single static binary** - No runtime dependencies, works anywhere
- ✅ **Small binary size** - ~683KB with full optimizations
- ✅ **Fast execution** - Near-instant startup, no interpreter overhead
- ✅ **Memory safety** - Rust's borrow checker prevents common bugs
- ✅ **Type safety** - Compile-time guarantees against null pointer issues
- ✅ **Easy distribution** - Download one file, run anywhere

## Building from Source

```bash
# Debug build (faster compilation, larger binary)
cargo build

# Release build (optimized, ~683KB)
cargo build --release

# Run tests
cargo test
bash scripts/test-rust.sh
```

**Build dependencies:** Rust toolchain (1.70+), `clap`, `clap_complete`
**Runtime dependencies:** None (static linking)

## Testing

Run the test suite to verify all functionality:

```bash
# Unit tests (Rust)
cargo test

# Integration tests (Shell)
bash scripts/test-rust.sh
```

**Test Coverage (99 tests):**

| Category | Tests |
|----------|-------|
| Unit Tests | config (6), permissions (20), symlink (8), path expansion (6) |
| CLI Commands | help, version, new, quiet, verbose |
| Config Parsing | valid/invalid TOML, empty config, comments, whitespace, quotes |
| Deploy | basic link, dry-run, tilde expansion, multiple links, force, unicode, absolute paths |
| Undeploy | remove symlink, nonexistent link |
| Validation | valid/invalid TOML, reject JSON, multiple files, auto-validation during deploy |
| Shell Completion | bash, zsh, fish, no shell, invalid shell |
| Permission Check | SSH key detection, fix, correct permission, pattern matching, dry-run |
| Symlink Safety | loop detection, circular scenario, parent symlink fix |
| Dependencies | subdirectory deployment |
| Interactive Mode | confirm yes/no |

## Container Testing

Test with isolated environment using bubblewrap:

```bash
./scripts/bwrap-test.sh
```

This runs a complete deployment test of `cncsmonster/dotfiles` in an isolated sandbox.

## Shell Completion

xdotter generates shell completion scripts at compile time using `clap_complete`.

### Quick Setup (Recommended)

One-line setup that works immediately:

```bash
# Bash - add to ~/.bashrc
eval "$(xd completion bash)"

# Zsh - add to ~/.zshrc
eval "$(xd completion zsh)"

# Fish - add to config.fish
xd completion fish | source
```

**Important:** Make sure `xd` is in your `PATH`. If you installed manually, add this to your `~/.bashrc` **before** the completion line:

```bash
export PATH="$HOME/.local/bin:$PATH"
eval "$(xd completion bash)"
```

### Alternative: Install Completion Files

If you prefer to install completion files:

```bash
# Bash
xd completion bash > ~/.local/share/bash-completion/completions/xd

# Zsh
xd completion zsh > ~/.local/share/zsh/site-functions/_xd

# Fish
xd completion fish > ~/.config/fish/completions/xd.fish
```

### Troubleshooting

If completion doesn't work:

1. **Check xd is in PATH**: `which xd` should return a path
2. **Reload your shell**: Run `source ~/.bashrc` (or `~/.zshrc`)
3. **Test manually**: Run `xd completion bash` to see the generated script
4. **Zsh specific**: Make sure `compinit` is loaded (the script does this automatically)

### How It Works

The `xd completion <shell>` command generates a shell script that:
1. Defines a completion function that calls `xd` with special environment variables
2. Registers the function with your shell's completion system
3. When you press TAB, the shell calls `xd` which uses clap_complete to generate completions dynamically

This ensures completions always stay in sync with the CLI definition - no manual maintenance needed.

## License

[MIT License](LICENSE)

## Contributing

Contributions are welcome! Feel free to open an issue or submit a PR.
