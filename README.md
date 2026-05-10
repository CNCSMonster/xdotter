# xdotter

xdotter is a small dotfile manager.

It creates symbolic links from files or directories in your dotfile repository to the places where programs expect them.

xdotter does **not** copy dotfile contents. Your real files stay in the repository; the target paths are symlinks pointing back to them.

## Example

```toml
[links]
".zshrc" = "~/.zshrc"
".config/nvim" = "~/.config/nvim"
```

Running:

```bash
xd deploy
```

creates links like:

```text
~/.zshrc        ->  <repo>/.zshrc
~/.config/nvim  ->  <repo>/.config/nvim
```

## Install

**cargo:**

```bash
cargo install xdotter
```

**cargo-binstall:**

```bash
cargo binstall xdotter
```

**Prebuilt binary (Linux/macOS/Windows):**

Download the asset matching your platform from the [latest release](https://github.com/CNCSMonster/xdotter/releases/latest)
and place it on your `PATH`:

```bash
# Linux / macOS
mkdir -p ~/.local/bin
curl -L https://github.com/CNCSMonster/xdotter/releases/latest/download/xd-$(rustc -vV | sed -n 's/host: //p') -o ~/.local/bin/xd
chmod +x ~/.local/bin/xd
```

If glibc is too old (e.g. Ubuntu 22.04), use the static musl binary instead:

```bash
curl -L https://github.com/CNCSMonster/xdotter/releases/latest/download/xd-x86_64-unknown-linux-musl -o ~/.local/bin/xd
chmod +x ~/.local/bin/xd
```

> `~/.local/bin` is part of the XDG Base Directory spec and is
> typically on `PATH` by default. If not, add `export PATH="$HOME/.local/bin:$PATH"`
> to your shell profile.

## Commands

```bash
xd deploy [--dry-run] [--force | --interactive]   # create configured symlinks
xd undeploy [--dry-run] [--force | --interactive] # remove configured symlinks
xd status                                         # show link status
xd new [--dry-run]                                # create a template xdotter.toml
xd completion <bash|zsh|fish>                     # generate shell completion
xd version                                        # print version
```

`-v` / `--verbose` is the only global option and may be repeated up to three times to increase log detail (`-v`, `-vv`, `-vvv`).

`--force` and `--interactive` are mutually exclusive.

## Configuration

xdotter uses `xdotter.toml` in the current directory.

`[links]` maps source paths to link paths. The TOML key is the source path inside this repository; the value is the link path (`~/...` or absolute) to create.

`[dependencies]` maps names to relative subdirectories that contain their own `xdotter.toml`.

```toml
[links]
".zshrc" = "~/.zshrc"

[dependencies]
"nvim" = "config/nvim"
"zsh"  = "modules/zsh"
```

Both tables are optional; an empty configuration is legal.

## Error classes

Error messages carry one of four classification prefixes:

- `[CLI 参数错误]` — invalid command-line usage.
- `[配置错误]` — `xdotter.toml` violates the static configuration rules.
- `[规划阻塞错误]` — planning could not safely build or apply a plan.
- `[应用阶段错误]` — error while applying a validated plan.

Exit code is `0` on success, non-zero on any failure.

## Design Source of Truth

Project behavior and safety rules are defined in [`SPEC.md`](SPEC.md). When this README and SPEC disagree, SPEC wins.
