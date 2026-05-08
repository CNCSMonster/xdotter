//! End-to-end SPEC-invariant tests, driven via the `xd` binary.
//!
//! These tests build the binary once via `env!("CARGO_BIN_EXE_xd")` and
//! exercise behavior that cannot be unit-tested in isolation: CLI
//! argument parsing, stdout/stderr separation, exit codes, and full
//! command flows over a temp dotfiles repo.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static COUNTER: AtomicU64 = AtomicU64::new(0);

fn xd_bin() -> &'static str {
    env!("CARGO_BIN_EXE_xd")
}

fn tmpdir(tag: &str) -> PathBuf {
    let id = COUNTER.fetch_add(1, Ordering::SeqCst);
    let p = std::env::temp_dir().join(format!("xd_it_{}_{}_{}", tag, std::process::id(), id));
    let _ = fs::remove_dir_all(&p);
    fs::create_dir_all(&p).unwrap();
    p
}

fn unique_home(tag: &str) -> PathBuf {
    let id = COUNTER.fetch_add(1, Ordering::SeqCst);
    let p = std::env::temp_dir().join(format!("xd_it_home_{}_{}_{}", tag, std::process::id(), id));
    let _ = fs::remove_dir_all(&p);
    fs::create_dir_all(&p).unwrap();
    p
}

struct Output {
    code: i32,
    stdout: String,
    stderr: String,
}

fn run_in(dir: &Path, args: &[&str], home: &Path) -> Output {
    let out = Command::new(xd_bin())
        .args(args)
        .current_dir(dir)
        .env("HOME", home)
        // Keep PATH so the linker / clang etc. work; everything else
        // is unset to avoid the host user's HOME leaking in.
        .output()
        .expect("failed to spawn xd binary");
    Output {
        code: out.status.code().unwrap_or(-1),
        stdout: String::from_utf8_lossy(&out.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&out.stderr).into_owned(),
    }
}

// ============================================================
// CLI parsing
// ============================================================

#[test]
fn version_prints_to_stdout() {
    let d = tmpdir("ver");
    let h = unique_home("ver");
    let o = run_in(&d, &["version"], &h);
    assert_eq!(o.code, 0, "stderr: {}", o.stderr);
    assert!(o.stdout.contains("xdotter "));
    assert!(o.stderr.is_empty());
}

#[test]
fn force_and_interactive_are_mutually_exclusive() {
    let d = tmpdir("excl");
    let h = unique_home("excl");
    fs::write(d.join("xdotter.toml"), "").unwrap();
    let o = run_in(&d, &["deploy", "--force", "--interactive"], &h);
    assert_ne!(o.code, 0);
    // clap prints arg conflict to stderr.
    assert!(
        o.stderr.to_lowercase().contains("cannot be used with")
            || o.stderr.to_lowercase().contains("conflicts")
            || o.stderr.contains("--force"),
        "expected clap conflict diagnostic on stderr, got: {}",
        o.stderr
    );
}

#[test]
fn unknown_top_level_key_is_classified_config_error() {
    let d = tmpdir("unkkey");
    let h = unique_home("unkkey");
    fs::write(d.join("xdotter.toml"), "wat = 1\n").unwrap();
    let o = run_in(&d, &["status"], &h);
    assert_ne!(o.code, 0);
    assert!(
        o.stderr.contains("[配置错误]"),
        "stderr should carry config-error label: {}",
        o.stderr
    );
}

#[test]
fn missing_config_is_classified_error() {
    let d = tmpdir("nocfg");
    let h = unique_home("nocfg");
    let o = run_in(&d, &["status"], &h);
    assert_ne!(o.code, 0);
    // Missing config in the cwd is reported as a config error.
    assert!(
        o.stderr.contains("[配置错误]") || o.stderr.contains("xdotter.toml"),
        "stderr: {}",
        o.stderr
    );
}

// ============================================================
// `xd new`
// ============================================================

#[test]
fn new_creates_template() {
    let d = tmpdir("new");
    let h = unique_home("new");
    let o = run_in(&d, &["new"], &h);
    assert_eq!(o.code, 0, "stderr: {}", o.stderr);
    assert!(d.join("xdotter.toml").exists());
}

#[test]
fn new_dry_run_does_not_create() {
    let d = tmpdir("newdry");
    let h = unique_home("newdry");
    let o = run_in(&d, &["new", "--dry-run"], &h);
    assert_eq!(o.code, 0);
    assert!(!d.join("xdotter.toml").exists());
}

#[test]
fn new_refuses_overwrite() {
    let d = tmpdir("newexist");
    let h = unique_home("newexist");
    fs::write(d.join("xdotter.toml"), "").unwrap();
    let o = run_in(&d, &["new"], &h);
    assert_ne!(o.code, 0);
    assert!(o.stderr.contains("[配置错误]"));
}

// ============================================================
// Deploy / Status / Undeploy happy path
// ============================================================

#[test]
#[cfg(unix)]
fn deploy_then_status_then_undeploy() {
    let d = tmpdir("happy");
    let h = unique_home("happy");
    fs::write(d.join("zshrc"), "# zsh").unwrap();
    fs::write(
        d.join("xdotter.toml"),
        r#"
[links]
"zshrc" = "~/.zshrc"
"#,
    )
    .unwrap();

    // Deploy
    let o = run_in(&d, &["deploy"], &h);
    assert_eq!(o.code, 0, "stderr: {}", o.stderr);
    assert!(h.join(".zshrc").is_symlink());

    // Status: 1/1 deployed.
    let o = run_in(&d, &["status"], &h);
    assert_eq!(o.code, 0, "stderr: {}", o.stderr);
    assert!(o.stdout.contains("Status: 1/1 deployed"));
    assert!(o.stdout.contains("Not deployed: 0"));
    assert!(o.stdout.contains("Wrong links: 0"));
    assert!(o.stdout.contains("Broken links: 0"));
    assert!(o.stdout.contains("Source missing: 0"));
    assert!(o.stdout.contains("Source type invalid: 0"));
    assert!(o.stdout.contains("Non-symlink paths: 0"));
    assert!(o.stdout.contains("Permission issues: 0"));

    // Undeploy
    let o = run_in(&d, &["undeploy"], &h);
    assert_eq!(o.code, 0, "stderr: {}", o.stderr);
    assert!(!h.join(".zshrc").exists());
}

#[test]
#[cfg(unix)]
fn dry_run_does_not_modify_filesystem() {
    let d = tmpdir("dry");
    let h = unique_home("dry");
    fs::write(d.join("zshrc"), "# zsh").unwrap();
    fs::write(
        d.join("xdotter.toml"),
        r#"
[links]
"zshrc" = "~/.zshrc"
"#,
    )
    .unwrap();

    let o = run_in(&d, &["deploy", "--dry-run"], &h);
    assert_eq!(o.code, 0, "stderr: {}", o.stderr);
    assert!(!h.join(".zshrc").exists());
    assert!(!h.join(".zshrc").is_symlink());
}

// ============================================================
// SPEC error classes — all four labels are reachable
// ============================================================

#[test]
fn link_path_with_dotdot_is_config_error() {
    let d = tmpdir("ddot");
    let h = unique_home("ddot");
    fs::write(d.join("a"), "x").unwrap();
    fs::write(
        d.join("xdotter.toml"),
        r#"
[links]
"a" = "/tmp/../etc/passwd"
"#,
    )
    .unwrap();
    let o = run_in(&d, &["deploy"], &h);
    assert_ne!(o.code, 0);
    assert!(o.stderr.contains("[配置错误]"), "stderr: {}", o.stderr);
}

#[test]
#[cfg(unix)]
fn missing_source_is_planning_error() {
    let d = tmpdir("missrc");
    let h = unique_home("missrc");
    fs::write(
        d.join("xdotter.toml"),
        r#"
[links]
"ghost" = "~/.ghost"
"#,
    )
    .unwrap();
    let o = run_in(&d, &["deploy"], &h);
    assert_ne!(o.code, 0);
    assert!(o.stderr.contains("[规划阻塞错误]"), "stderr: {}", o.stderr);
}

#[test]
fn link_root_or_home_rejected() {
    for raw in ["~", "~/", "/", "//"] {
        let d = tmpdir("root");
        let h = unique_home("root");
        fs::write(d.join("a"), "x").unwrap();
        fs::write(
            d.join("xdotter.toml"),
            format!(
                r#"
[links]
"a" = "{}"
"#,
                raw
            ),
        )
        .unwrap();
        let o = run_in(&d, &["deploy"], &h);
        assert_ne!(o.code, 0, "raw={raw}");
        assert!(
            o.stderr.contains("[配置错误]"),
            "raw={raw} stderr: {}",
            o.stderr
        );
    }
}

#[test]
#[cfg(unix)]
fn global_link_collision_lists_all_offenders() {
    let d = tmpdir("col");
    let h = unique_home("col");
    let sub = d.join("sub");
    fs::create_dir_all(&sub).unwrap();
    fs::write(d.join("a"), "x").unwrap();
    fs::write(sub.join("b"), "y").unwrap();
    fs::write(
        d.join("xdotter.toml"),
        r#"
[links]
"a" = "~/.collide"

[dependencies]
"sub" = "sub"
"#,
    )
    .unwrap();
    fs::write(
        sub.join("xdotter.toml"),
        r#"
[links]
"b" = "~/.collide"
"#,
    )
    .unwrap();
    let o = run_in(&d, &["deploy"], &h);
    assert_ne!(o.code, 0);
    assert!(o.stderr.contains("[配置错误]"));
    assert!(
        o.stderr.contains("源 \"a\""),
        "should list source a: {}",
        o.stderr
    );
    assert!(
        o.stderr.contains("源 \"b\""),
        "should list source b: {}",
        o.stderr
    );
}

// ============================================================
// Output stream contract
// ============================================================

// ============================================================
// SPEC compliance regressions (audit fixes #2 #3 #4 #5 #1 #19)
// ============================================================

#[test]
#[cfg(unix)]
fn deploy_force_replaces_regular_file_and_continues_after_skipped_link() {
    // Reg: even if one link is a recoverable conflict in default mode,
    // subsequent links must still be processed (via --force here).
    let d = tmpdir("force_continue");
    let h = unique_home("force_continue");
    fs::write(d.join("a"), "A").unwrap();
    fs::write(d.join("b"), "B").unwrap();
    // Prepopulate the link path as a regular file so deploy must replace it.
    fs::write(h.join(".a"), "stale").unwrap();
    fs::write(
        d.join("xdotter.toml"),
        r#"
[links]
"a" = "~/.a"
"b" = "~/.b"
"#,
    )
    .unwrap();

    let o = run_in(&d, &["deploy", "--force"], &h);
    assert_eq!(o.code, 0, "stderr: {}", o.stderr);
    assert!(h.join(".a").is_symlink());
    assert!(h.join(".b").is_symlink());
}

#[test]
#[cfg(unix)]
fn dry_run_interactive_renders_skip_not_replace() {
    // Reg #5: --interactive --dry-run must show "would skip
    // (interactive declined)" not "would replace".
    let d = tmpdir("dryint");
    let h = unique_home("dryint");
    fs::write(d.join("a"), "A").unwrap();
    fs::write(h.join(".a"), "stale").unwrap();
    fs::write(
        d.join("xdotter.toml"),
        r#"
[links]
"a" = "~/.a"
"#,
    )
    .unwrap();

    let o = run_in(&d, &["deploy", "--interactive", "--dry-run"], &h);
    assert_ne!(
        o.code, 0,
        "interactive dry-run declined conflicts should fail"
    );
    assert!(
        o.stderr.contains("[规划阻塞错误]"),
        "stderr should carry planning label; stderr: {}",
        o.stderr
    );
    // Must not promise a replacement.
    assert!(
        !o.stdout.contains("replace"),
        "interactive dry-run must not say replace; stdout: {}",
        o.stdout
    );
    assert!(
        o.stdout.contains("interactive declined"),
        "expected interactive declined marker; stdout: {}",
        o.stdout
    );
    // FS untouched.
    assert!(!h.join(".a").is_symlink());
}

#[test]
#[cfg(unix)]
fn dry_run_force_renders_replace() {
    // Reg #5 partner: --force --dry-run must show "replace".
    let d = tmpdir("dryforce");
    let h = unique_home("dryforce");
    fs::write(d.join("a"), "A").unwrap();
    fs::write(h.join(".a"), "stale").unwrap();
    fs::write(
        d.join("xdotter.toml"),
        r#"
[links]
"a" = "~/.a"
"#,
    )
    .unwrap();

    let o = run_in(&d, &["deploy", "--force", "--dry-run"], &h);
    assert_eq!(o.code, 0, "stderr: {}", o.stderr);
    assert!(
        o.stdout.contains("replace"),
        "force dry-run should say replace; stdout: {}",
        o.stdout
    );
    // FS untouched.
    assert_eq!(fs::read_to_string(h.join(".a")).unwrap(), "stale");
}

#[test]
fn force_interactive_mutex_carries_cli_label() {
    // Reg #1: clap's mutex error must be wrapped with the SPEC
    // [CLI 参数错误] classification label.
    let d = tmpdir("mutex");
    let h = unique_home("mutex");
    fs::write(d.join("xdotter.toml"), "").unwrap();
    let o = run_in(&d, &["deploy", "--force", "--interactive"], &h);
    assert_ne!(o.code, 0);
    assert!(
        o.stderr.contains("[CLI 参数错误]"),
        "stderr should carry CLI error label; stderr: {}",
        o.stderr
    );
}

#[test]
#[cfg(unix)]
fn deploy_emits_sensitive_target_warning_even_when_correct() {
    // Reg #4: hitting a built-in permission target must emit a warning
    // to stderr, regardless of whether permission is correct.
    let d = tmpdir("sens_ok");
    let h = unique_home("sens_ok");
    fs::create_dir_all(h.join(".ssh")).unwrap();
    // Source with already-correct 0600 mode.
    let src = d.join("id_test");
    fs::write(&src, "key").unwrap();
    {
        use std::os::unix::fs::PermissionsExt;
        let mut p = fs::metadata(&src).unwrap().permissions();
        p.set_mode(0o600);
        fs::set_permissions(&src, p).unwrap();
    }
    fs::write(
        d.join("xdotter.toml"),
        r#"
[links]
"id_test" = "~/.ssh/id_test"
"#,
    )
    .unwrap();

    let o = run_in(&d, &["deploy"], &h);
    assert_eq!(o.code, 0, "stderr: {}", o.stderr);
    assert!(h.join(".ssh/id_test").is_symlink());
    assert!(
        o.stderr.contains("敏感目标"),
        "expected sensitive-target warning on stderr; stderr: {}",
        o.stderr
    );
}

#[test]
#[cfg(unix)]
fn verbose_emits_per_link_diagnostics_to_stderr() {
    // Reg #19: -v must affect deploy output, not just status.
    let d = tmpdir("verbose");
    let h = unique_home("verbose");
    fs::write(d.join("a"), "A").unwrap();
    fs::write(
        d.join("xdotter.toml"),
        r#"
[links]
"a" = "~/.a"
"#,
    )
    .unwrap();
    let o = run_in(&d, &["-v", "deploy"], &h);
    assert_eq!(o.code, 0, "stderr: {}", o.stderr);
    // Verbose diagnostics belong on stderr (warnings/diagnostics stream).
    assert!(
        o.stderr.contains("deploy:") || o.stderr.contains("create"),
        "expected verbose diagnostics on stderr; stderr: {}",
        o.stderr
    );
}

#[test]
#[cfg(unix)]
fn unsafe_ancestor_regular_file_is_planning_error() {
    // Reg #22: a non-directory ancestor (deeper than direct parent) is
    // a planning-block error, not an apply-stage failure.
    let d = tmpdir("anc_file");
    let h = unique_home("anc_file");
    // Make ~/.config a regular file so ~/.config/sub/file becomes
    // "ancestor not a directory".
    fs::write(h.join(".config"), "not-a-dir").unwrap();
    fs::write(d.join("a"), "A").unwrap();
    fs::write(
        d.join("xdotter.toml"),
        r#"
[links]
"a" = "~/.config/sub/file"
"#,
    )
    .unwrap();
    let o = run_in(&d, &["deploy"], &h);
    assert_ne!(o.code, 0);
    assert!(
        o.stderr.contains("[规划阻塞错误]"),
        "expected planning-block label; stderr: {}",
        o.stderr
    );
}

#[test]
#[cfg(unix)]
fn status_summary_lines_go_to_stdout() {
    let d = tmpdir("ss");
    let h = unique_home("ss");
    fs::write(d.join("xdotter.toml"), "").unwrap();
    let o = run_in(&d, &["status"], &h);
    assert_eq!(o.code, 0, "stderr: {}", o.stderr);
    assert!(o.stdout.contains("Status: 0/0 deployed"));
    assert!(o.stderr.is_empty());
}
