# xdotter Rust Rewrite Plan

> Rewriting xdotter from Python to Rust for reliable single-binary distribution.

## Goals

- [x] Single static binary (~500KB target)
- [x] Clear errors (no "null bytes" confusion)
- [x] Pass all 50 existing test cases
- [x] Compatible CLI interface

## Test Coverage Target (58 tests)

### CLI Basics (5 tests)
- [ ] Help Command
- [ ] Version Command
- [ ] New Command
- [ ] Quiet Mode
- [ ] Verbose Mode

### Config Parsing (6 tests)
- [ ] Config Parsing
- [ ] Invalid TOML Syntax
- [ ] Empty Config
- [ ] Comments In Config
- [ ] Whitespace In Config
- [ ] Single Quotes In Config

### Deploy (8 tests)
- [ ] Deploy Basic Link
- [ ] Deploy Dry Run
- [ ] Tilde Expansion
- [ ] Multiple Links
- [ ] Force Flag
- [ ] Symlink Already Exists
- [ ] Unicode Paths
- [ ] Absolute Path In Config

### Undeploy (2 tests)
- [ ] Undeploy
- [ ] Undeploy Nonexistent Link

### Dependencies (1 test)
- [ ] Dependencies Subdirectory

### Interactive Mode (2 tests)
- [ ] Interactive Mode Confirm
- [ ] Interactive Mode Yes

### Error Handling (2 tests)
- [ ] Nonexistent Source
- [ ] Nonexistent Config

### Validation (7 tests)
- [ ] Validate Valid TOML
- [ ] Validate Invalid TOML
- [ ] Validate Valid JSON
- [ ] Validate Invalid JSON
- [ ] Validate Nonexistent File
- [ ] Validate Multiple Files
- [ ] Validate Default Files
- [ ] Deploy Auto-Validation Invalid
- [ ] Deploy No Validate Flag
- [ ] Deploy Auto-Validation Valid

### Shell Completion (5 tests)
- [ ] Completion Bash
- [ ] Completion Zsh
- [ ] Completion Fish
- [ ] Completion No Shell
- [ ] Completion Invalid Shell

### Permissions (5 tests)
- [ ] Permission Check SSH Key
- [ ] Permission Fix SSH Key
- [ ] Permission Check Correct
- [ ] Permission Pattern Matching
- [ ] Permission Dry Run

### Symlink Safety (5 tests)
- [ ] Symlink Loop Detection
- [ ] Deploy Symlink Loop Warning
- [ ] Circular Symlink Scenario
- [ ] Force Fixes Parent Symlink

## Progress

| Commit | Description | Tests Passing |
|--------|-------------|---------------|
| - | Initial Rust project setup | 0/58 |
| - | CLI argument parsing | 5/58 |
| - | Config parsing (TOML/JSON) | 11/58 |
| - | Basic deploy | 19/58 |
| - | Undeploy | 21/58 |
| - | Dry run, force, verbose | 26/58 |
| - | Validation command | 33/58 |
| - | Shell completion | 38/58 |
| - | Permission checking | 43/58 |
| - | Symlink loop detection | 48/58 |
| - | Edge cases, interactive | 58/58 |

## Tech Stack

- **CLI**: `clap` (argparse equivalent)
- **TOML**: `toml` crate
- **JSON**: `serde_json`
- **Paths**: `std::path::PathBuf`
- **Symlinks**: `std::os::unix::fs::symlink`
- **Shell completion**: `clap_complete`

## Build Configuration

```toml
[profile.release]
opt-level = "z"
lto = true
strip = true
codegen-units = 1
```
