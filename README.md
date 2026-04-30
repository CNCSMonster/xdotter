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
~/.config/nvim ->  <repo>/.config/nvim
```

## Commands

```bash
xd deploy      # create configured symlinks
xd undeploy    # remove configured symlinks
xd status      # show link status
xd validate    # validate xdotter.toml
xd new         # create a template xdotter.toml
xd completion  # generate shell completion
xd version     # print version
```

## Configuration

xdotter uses `xdotter.toml` in the current directory.

`[links]` maps source paths to target symlink paths.

`[dependencies]` maps names to relative subdirectories with their own `xdotter.toml`.

```toml
[dependencies]
"nvim" = "config/nvim"
"zsh" = "modules/zsh"
```

Dependency paths are relative subdirectories under the current configuration directory.

## Design Source of Truth

Project behavior and safety rules are defined in [`SPEC.md`](SPEC.md).
