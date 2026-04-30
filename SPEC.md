# xdotter Specification

## Status

This document is the source of truth for xdotter's behavior and design. Code, tests, README, and other documentation should be updated to match this specification.

## Objective

xdotter is a small dotfiles/environment configuration deployment tool. Its job is to create and remove symbolic links from files or directories in a user-controlled dotfile repository to target locations such as the user's `$HOME`.

The core model is simple:

```text
source file or directory in repo  ->  symbolic link at target path
```

xdotter does not copy file contents. It creates symlinks.

## Scope and Non-goals

xdotter is designed for deploying a user's own dotfiles into that user's own environment, such as `$HOME`, `~/.config`, shell configuration files, SSH configuration, and similar user-controlled paths.

xdotter is not a sandbox, privilege boundary, package manager, backup system, or multi-user filesystem isolation tool.

xdotter does not guarantee safe behavior when deployment targets are concurrently modified by untrusted users or processes during deployment.

Users should avoid deploying links into world-writable or otherwise untrusted directories.

## Supported Platforms

- Linux
- macOS
- Windows support exists through platform-specific symlink APIs, but behavior may depend on OS permissions.

## Configuration Format

xdotter uses a fixed TOML configuration file named:

```text
xdotter.toml
```

Only TOML is part of the current supported user-facing configuration format.

Example:

```toml
[links]
".zshrc" = "~/.zshrc"
".config/nvim" = "~/.config/nvim"

[dependencies]
"nvim" = "config/nvim"
```

### `[links]`

The `[links]` table maps:

```text
source = link
```

- `source` is the real file or directory to expose.
- `link` is the symlink path to create.

### `[dependencies]`

The `[dependencies]` table maps dependency names to relative subdirectories that contain their own `xdotter.toml` files.

## Configuration Trust Model

`xdotter.toml` is deployment configuration that may cause filesystem writes, symlink creation, file removal, directory removal, and permission changes.

Paths from configuration must be validated according to their role before destructive filesystem operations.

`--force` must not skip configuration safety validation.

`--dry-run` must perform the same safety validation as the corresponding real command, even though it must not modify the filesystem.

`--no-validate` may skip syntax validation, but it must not skip path safety, dependency safety, or symlink topology checks.

## Validation and Operation Planning

xdotter separates configuration validation from operation preflight.

`xd validate` validates `xdotter.toml` as configuration. It checks TOML syntax and static configuration rules that do not require applying a deployment or undeployment plan.

`xd deploy --dry-run` is the preflight mode for deployment. It must run the same validation and safety checks as `xd deploy`, inspect the filesystem state needed for deployment, build the deployment plan, print what would happen, and stop before modifying the filesystem.

`xd undeploy --dry-run` is the preflight mode for undeployment. It must run the same validation and safety checks as `xd undeploy`, inspect the filesystem state needed for undeployment, build the undeployment plan, print what would happen, and stop before modifying the filesystem.

A separate deployment check command is not part of the current CLI model. Use `xd deploy --dry-run` or `xd undeploy --dry-run` for operation preflight.

Syntax validation covers TOML parsing, table structure, and key types. It does not require filesystem state.

Safety validation covers path role rules (link path parent traversal, dependency path escape), symlink topology checks, filesystem identity checks, and permission safety. Safety validation always runs before destructive operations.

Command execution follows this lifecycle:

1. load configuration
2. validate configuration syntax and static path rules
3. inspect filesystem state required by the command
4. build an operation plan
5. run safety checks for that plan
6. if `--dry-run`: print the plan and stop
7. otherwise: apply the plan

## Path Semantics

### `~` Expansion

Paths beginning with `~/` expand to the current user's home directory.

### Source Paths

Source paths may be absolute, home-relative, or relative to the current configuration directory.

A source path must exist before deployment.

Source paths may contain parent-directory traversal (`..`) only if the resolved source remains valid and does not create invalid link/source topology.

### Link Paths

Link paths describe filesystem locations that xdotter may create, remove, or replace. Therefore link paths are destructive-operation targets and are treated more strictly than source paths.

Rules:

- Link paths may be absolute, home-relative, or relative to the current configuration directory.
- Link paths must not contain parent-directory traversal (`..`) in any path component after `~` expansion.
- If a link path contains `..`, deployment must fail before any filesystem modification.
- This rule applies regardless of `--force`, `--interactive`, `--dry-run`, or `--no-validate`.

Rationale: link paths are destructive operation targets. Allowing `..` makes the visible path differ from the actual operation location and complicates safety checks.

### Dependency Paths

Dependency paths identify subdirectories containing their own `xdotter.toml` files.

Dependency paths must be relative subdirectories under the current configuration directory.

Dependency paths must not be absolute.

Dependency paths must not contain parent-directory traversal (`..`).

After resolving symlinks, a dependency directory must still be inside the current configuration directory tree.

If a dependency path escapes the current configuration directory, deployment must fail before entering the dependency directory or modifying the filesystem.

### Filesystem Identity

xdotter must not rely on string equality alone when checking whether two paths refer to the same filesystem object.

For existing paths, xdotter should compare resolved filesystem identity where practical.

At minimum, before destructive operations, xdotter must reject:

- source and link resolving to the same filesystem object
- link located inside source
- source located inside an existing real link directory that would be removed

## Symlink Safety Semantics

xdotter must prevent invalid link/source topology.

The following cases are invalid and must fail even with `--force`:

- source and link resolve to the same path
- link is inside source
- source is inside an existing real link directory that would be removed
- creating the symlink would create a symlink loop
- creating the symlink would create a circular symlink scenario

A correctly existing symlink that already points to the intended source should be treated as success and skipped.

## Destructive Operation Semantics

### `--force`

`--force` changes how xdotter handles recoverable conflicts at the link path. It must not weaken path validation, dependency validation, permission safety, or invalid topology checks.

Allowed recoverable cases include:

- replacing an existing regular file at the link path
- replacing an existing wrong symlink at the link path
- replacing an existing directory at the link path, if doing so cannot remove the source
- repairing a parent symlink when the existing parent symlink would otherwise cause deployment to target the source location itself

Forbidden even with `--force`:

- source equals link
- link inside source
- source inside a link directory that would be removed
- link path containing `..`
- dependency path escaping the current configuration directory
- symlink loop or circular symlink topology
- permission fixing when the target cannot be verified as the expected source

### `--interactive`

`--interactive` may ask the user to confirm recoverable destructive operations.

`--interactive` must not make invalid topology, invalid paths, or unsafe dependency traversal valid.

### `--dry-run`

`--dry-run` must not modify the filesystem.

`--dry-run` must perform the same path validation, dependency validation, topology checks, and permission safety checks as the corresponding real command.

Invalid configurations and unsafe operation plans must fail in dry-run mode. Dry-run must not describe an invalid or unsafe operation as if it would be performed.

In dry-run mode, xdotter may report intended operations, but must not:

- create files or directories
- remove files, directories, or symlinks
- create symlinks
- change permissions

### Race Safety and Fail-Closed Behavior

xdotter assumes deployment targets are normally located in user-controlled directories.

Adversarial concurrent mutation of deployment paths by untrusted users or processes is not a supported use case.

However, before destructive operations, xdotter must validate the target state relevant to that operation.

If the target state is missing, ambiguous, or no longer satisfies the safety preconditions, xdotter must fail closed rather than continue.

Destructive operations include:

- removing an existing link path
- recursively removing an existing directory at a link path
- replacing an existing path
- removing a parent symlink as a repair action
- changing permissions

## Permission Semantics

xdotter may check or fix permissions for sensitive target paths during deployment.

Permission checks are based on the link target path, not only the source filename.

Permission checks must fail closed. If xdotter cannot read metadata for a path that requires permission checking, it must report a permission issue instead of treating the path as valid.

Before fixing permissions, xdotter must verify that the link still resolves to the expected source.

If the link no longer resolves to the expected source, or the expected target cannot be verified, permission fixing must fail without changing permissions.

### SSH Keys

SSH private keys should use restrictive permissions such as `0600`.

SSH public keys may use `0644`, but only when the file content looks like an SSH public key.

A `.pub` filename alone is not enough to classify a file as public. If a `.pub` file with an SSH key-like name cannot be read or does not look like a public key, xdotter must fail closed and treat it as private-key sensitive.

## Dependency Semantics

Deploying a configuration should recursively deploy dependency configurations declared under `[dependencies]`.

Dependency traversal must remain within the current configuration directory tree according to Dependency Path rules.

Deploy and undeploy must apply the same dependency path validation rules before traversing dependency configurations.

Dependency traversal must detect true cycles. A shared dependency in a dependency graph is not necessarily a cycle.

Undeploy behavior should remain consistent with deploy behavior when dependencies are supported.

## Commands

### `xd deploy`

Default command. Reads `xdotter.toml`, validates configuration and path safety, deploys each link and recursively processes dependencies.

Detailed deploy behavior is defined by:
- Configuration Trust Model
- Path Semantics (source, link, dependency, identity)
- Symlink Safety Semantics
- Destructive Operation Semantics (force, interactive, dry-run)
- Permission Semantics
- Dependency Semantics

### `xd undeploy`

Reads `xdotter.toml` and removes symlinks that match the configured links.

Behavior by link path state:

| Link path state | Default | `--force` | `--interactive` | `--dry-run` |
|---|---|---|---|---|
| Is a symlink | Remove it | Remove it | Ask before removing | Print "Would remove" |
| Exists but is not a symlink | Warning, count as failure | Warning, count as failure (still continues) | Warning, ask before removing | Print "Would remove (not a symlink)" |
| Does not exist | Silent skip | Silent skip | Silent skip | Print "Would skip (not deployed)" |

Undeploy must apply the same dependency path validation rules as deploy before traversing dependency configurations.

### `xd status`

Reads `xdotter.toml` and reports the current state of each configured link.

Status classification:

- **Deployed**: link path is a symlink pointing to a target that exists on disk
- **Broken**: link path is a symlink but the target does not exist
- **Not a symlink**: link path exists as a regular file or directory (not a symlink)
- **Not deployed**: link path does not exist

If `--check-permissions` is set, status must also check permissions on deployed links and report any permission issues.

If `--verbose` is set, status must print all link paths including correct ones.

Summary format at end of output:

```text
Status: N/M deployed
Broken links: N
Permission issues: N
```

### `xd validate`

Validates `xdotter.toml` as configuration. Checks TOML syntax and static configuration rules that do not require applying a deployment or undeployment plan.

See Validation and Operation Planning for the full semantics.

### `xd new`

Creates a template `xdotter.toml` in the current directory with commented-out example sections for `[links]` and `[dependencies]`.

If `xdotter.toml` already exists, must fail with an error.

### `xd completion <shell>`

Generates shell completion scripts.

Supported shells: `bash`, `zsh`, `fish`.

The completion script is printed to stdout for the user to source or redirect to a completion directory.

### `xd version`

Prints the xdotter version number to stdout.

## Global Flags

```text
-v, --verbose      Show more information (debug-level output)
-q, --quiet        Suppress informational output; errors still go to stderr
-n, --dry-run      Preview operations without modifying the filesystem
-i, --interactive  Ask for confirmation before destructive operations
-f, --force        Allow overwriting existing targets at link paths
--check-permissions  Check sensitive file permissions during deploy or status
--fix-permissions    Fix sensitive file permissions during deploy
--no-validate        Skip syntax validation during deploy
```

## Exit Codes

- `0`: Success — all operations completed without errors
- `1`: Error — one or more operations failed; details printed to stderr

## Testing Requirements

Behavioral changes must include regression tests.

Required checks before considering a change complete:

```bash
cargo fmt
cargo test
cargo clippy --all-targets -- -D warnings
./scripts/test-rust.sh
```

Tests that mutate process-global state such as `HOME` or current working directory must be isolated to avoid flaky parallel test behavior.

Path safety tests must cover:

- link paths containing `..`
- dependency paths that are absolute
- dependency paths containing `..`
- dependency paths that resolve outside the current configuration directory through symlinks
- `xd validate` checking static configuration rules
- `xd deploy --dry-run` acting as deployment preflight without modifying the filesystem
- `xd undeploy --dry-run` acting as undeployment preflight without modifying the filesystem
- `--force`, `--interactive`, `--dry-run`, and `--no-validate` not bypassing safety checks

Permission safety tests must cover:

- metadata read failures being reported as permission issues
- permission fixing refusing to modify permissions when the expected source cannot be verified

## Documentation Rules

- `SPEC.md` is the design source of truth.
- `README.md` should stay minimal and user-facing.
- `README.md` should describe xdotter as a symlink-based dotfile manager.
- User-facing documentation should describe currently supported behavior only.
- Do not mention unsupported configuration formats in current user-facing docs unless the topic is explicit error behavior.

## Boundaries

### Always

- Validate configuration safety before destructive deployment.
- Run safety checks before destructive operations.
- Keep `--dry-run` non-mutating.
- Keep xdotter scoped as a simple dotfiles/environment deployment tool.
- Add tests for bug fixes and behavior changes.

### Ask First

- Adding a new configuration format.
- Changing `--force` semantics.
- Changing destructive filesystem behavior.
- Adding new runtime dependencies.
- Changing CI/release workflows.
- Expanding xdotter into backup, package-management, sandboxing, or multi-user isolation responsibilities.

### Never

- Commit secrets.
- Silently delete source files because of a configuration mistake.
- Let `--force` override invalid link/source topology.
- Let `--force` bypass path or dependency safety checks.
- Let `--dry-run` mutate the filesystem.
- Claim xdotter provides sandboxing, privilege isolation, or adversarial multi-user filesystem safety.
