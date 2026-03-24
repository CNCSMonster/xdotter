# xdotter: a simple dotfile manager

A **zero-dependency**, **single-file** dotfile manager written in Python. No build tools, no package managers—just download and run.

## Features

- ✅ **Zero dependencies** - Uses Python standard library only
- ✅ **Single file** - Easy to distribute and understand
- ✅ **Cross-platform** - Works on Linux, macOS, and Windows (with Python)
- ✅ **No installation required** - Run directly or install with one command
- ✅ **Fast & Simple** - Minimal overhead, easy to configure

## Quick Start

### Option 1: Install (Recommended)

```bash
# Downloads to ~/.local/bin/xd
curl -sSL https://raw.githubusercontent.com/cncsmonster/xdotter/main/install.sh | bash

# Add to PATH (if not already)
export PATH="$HOME/.local/bin:$PATH"
```

### Option 2: Download & Run Directly

```bash
# Download the script
curl -sSL https://raw.githubusercontent.com/cncsmonster/xdotter/main/xd.py -o xd.py
chmod +x xd.py

# Run it
./xd.py --help
```

### Option 3: Clone Repository

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

# Use a custom config file
xd -c myconfig.toml deploy

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
| `new` | Create a new `xdotter.toml` template |
| `help` | Print help message |
| `version` | Print version |

### Options

| Option | Description |
|--------|-------------|
| `-c, --config <FILE>` | Specify configuration file [default: `xdotter.toml`] |
| `-v, --verbose` | Show more information |
| `-q, --quiet` | Do not print any output |
| `-n, --dry-run` | Show what would be done without making changes |
| `-i, --interactive` | Ask for confirmation when unsure |
| `-f, --force` | Force overwrite existing files |

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
- ✅ Single file, easy to audit and modify
- ✅ Download and use immediately

## Requirements

- Python 3.6+
- Unix-like system (Linux, macOS) or Windows with Python

## Testing

Run the test suite to verify all functionality:

```bash
python3 test_xd.py
```

**Test Coverage (32 tests):**

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

## Container Testing

Test with isolated environment using bubblewrap:

```bash
./bwrap-test.sh
```

This runs a complete deployment test of `cncsmonster/dotfiles` in an isolated sandbox.

See [TEST_REPORT.md](TEST_REPORT.md) for detailed results.

## License

[MIT License](LICENSE)

## Contributing

Contributions are welcome! Feel free to open an issue or submit a PR.
