//! Apply stage per SPEC §"预演或应用" + §"竞态安全".
//!
//! Executes the actions produced by `plan`. Before each destructive
//! operation, re-checks the filesystem state observed at planning time;
//! if it has changed, fails closed instead of proceeding. Apply-stage
//! re-checks run only in apply mode (not in --dry-run) per
//! `dry_run_no_apply_recheck`.
//!
//! Per-step result is three-state per SPEC:
//!   - Success — link processed.
//!   - SkippedFailure — recoverable conflict, user reject, or
//!     not-a-symlink warning. Counts as a failure in the totals
//!     but the loop continues with the next link.
//!   - HardFailure — apply-stage system error (OS error,
//!     re-check mismatch, OS-level permission fix failure).
//!     The loop stops immediately per §"应用阶段错误".

use std::fs;
use std::io::{self, BufRead, IsTerminal, Write};
use std::path::Path;

use crate::error::{ErrorBag, XdError};
use crate::permissions;
use crate::plan::{
    any_symlink_component, describe_existing, DeployAction, DeployActionKind, DeployPlan,
    ExistingKind, PermissionAction, UndeployAction, UndeployActionKind, UndeployPlan,
};

#[cfg(unix)]
use std::os::unix::fs as unix_fs;

/// Result of applying a plan: per-action outcome plus aggregate errors.
#[derive(Debug, Default)]
pub struct ApplyOutcome {
    pub successes: usize,
    pub skipped: usize,
    pub failures: usize,
    pub errors: ErrorBag,
}

/// Generic per-step result. Used by both deploy and undeploy.
enum StepResult {
    Success,
    /// Link skipped (recoverable conflict / user reject / warning) —
    /// counted as a failure, but the loop continues.
    SkippedFailure(XdError),
    /// Apply-stage system failure — counted as a failure and the loop
    /// stops on the spot.
    HardFailure(XdError),
}

// -----------------------------------------------------------------------------
// Deploy
// -----------------------------------------------------------------------------

pub fn apply_deploy(plan: &DeployPlan) -> ApplyOutcome {
    let mut out = ApplyOutcome::default();
    for act in &plan.actions {
        // Sensitive-target warning per SPEC §"权限和敏感文件语义":
        // independent of permission state — emitted whenever the link
        // hits a built-in permission target.
        emit_sensitive_warning(act);

        match apply_one_deploy(act, plan.mode.interactive) {
            StepResult::Success => out.successes += 1,
            StepResult::SkippedFailure(e) => {
                out.failures += 1;
                out.errors.push(e);
                // continue with next link
            }
            StepResult::HardFailure(e) => {
                out.failures += 1;
                out.errors.push(e);
                break;
            }
        }
    }
    out
}

fn emit_sensitive_warning(act: &DeployAction) {
    if let Some((mode, label)) = act.permission_required {
        eprintln!(
            "[警告] 链接 {} 命中敏感目标 ({}, 期望权限 {:o})；请确认该路径由 xdotter 管理",
            act.link_expanded.display(),
            label,
            mode
        );
    }
}

fn apply_one_deploy(act: &DeployAction, interactive: bool) -> StepResult {
    let link = &act.link_expanded;
    let source = &act.source_canonical;

    // Decide top-level action.
    match &act.kind {
        DeployActionKind::AlreadyCorrect => {
            // Permission step still applies.
            handle_permission(act, interactive)
        }
        DeployActionKind::SkipFailure(reason) => StepResult::SkippedFailure(XdError::planning(
            format!("链接 {} 因可恢复冲突跳过: {}", link.display(), reason),
        )),
        DeployActionKind::Create => {
            // Apply-stage re-check: source path must not have had symlink
            // components injected between planning and apply (TOCTOU defence).
            if any_symlink_component(&act.source_canonical, &act.config_dir) {
                return StepResult::HardFailure(XdError::apply(format!(
                    "应用阶段重新校验失败: 源路径 {} 出现了符号链接组件",
                    act.source_canonical.display()
                )));
            }
            if let Err(e) = ensure_parent_dir(link) {
                return StepResult::HardFailure(e);
            }
            if let Err(e) = recheck_link_missing(link) {
                return StepResult::HardFailure(e);
            }
            if let Err(e) = create_symlink(link, source) {
                return StepResult::HardFailure(XdError::apply(format!(
                    "创建符号链接失败 {} -> {}: {}",
                    link.display(),
                    source.display(),
                    e
                )));
            }
            handle_permission(act, interactive)
        }
        DeployActionKind::Replace(existing) => {
            // Interactive: prompt for the destructive replace.
            if interactive {
                let prompt = format!(
                    "替换 {} ({})? [y/N] ",
                    link.display(),
                    describe_existing(existing)
                );
                if !confirm(&prompt) {
                    // User reject = SkippedFailure (continue with next link).
                    return StepResult::SkippedFailure(XdError::planning(format!(
                        "链接 {} 在交互确认时被拒绝",
                        link.display()
                    )));
                }
            }
            if let Err(e) = recheck_existing_kind(link, existing) {
                if matches!(existing, ExistingKind::EmptyRealDir) {
                    // Empty dir became non-empty between plan and apply:
                    // treat as recoverable skip rather than hard failure.
                    return StepResult::SkippedFailure(XdError::planning(format!(
                        "{} 在规划后变为非空目录，跳过",
                        link.display()
                    )));
                }
                return StepResult::HardFailure(e);
            }
            // Apply-stage re-check: source path must not have had symlink
            // components injected between planning and apply.
            if any_symlink_component(&act.source_canonical, &act.config_dir) {
                return StepResult::HardFailure(XdError::apply(format!(
                    "应用阶段重新校验失败: 源路径 {} 出现了符号链接组件",
                    act.source_canonical.display()
                )));
            }
            if let Err(e) = remove_existing(link, existing) {
                return StepResult::HardFailure(e);
            }
            if let Err(e) = ensure_parent_dir(link) {
                return StepResult::HardFailure(e);
            }
            if let Err(e) = create_symlink(link, source) {
                return StepResult::HardFailure(XdError::apply(format!(
                    "创建符号链接失败 {} -> {}: {}",
                    link.display(),
                    source.display(),
                    e
                )));
            }
            handle_permission(act, interactive)
        }
    }
}

fn handle_permission(act: &DeployAction, interactive: bool) -> StepResult {
    match (&act.permission_action, &act.permission_required) {
        (PermissionAction::None, _) | (PermissionAction::AlreadyOk, _) => StepResult::Success,
        (PermissionAction::SkipFailure(reason), _) => {
            StepResult::SkippedFailure(XdError::planning(format!(
                "权限问题: {} ({})",
                act.link_expanded.display(),
                reason
            )))
        }
        (PermissionAction::Fix, Some((mode, label))) => {
            if interactive {
                let prompt = format!(
                    "修复 {} 权限为 {:o}? [y/N] ",
                    act.link_expanded.display(),
                    mode
                );
                if !confirm(&prompt) {
                    // User reject = SkippedFailure: per SPEC interactive
                    // granularity, rejecting any required destructive
                    // operation skips the entire link, but we move on
                    // to the next link rather than abort.
                    return StepResult::SkippedFailure(XdError::planning(format!(
                        "链接 {} 的权限修复在交互确认时被拒绝",
                        act.link_expanded.display()
                    )));
                }
            }
            // Apply-stage re-check: target must still resolve to the configured source.
            // Also re-verify source path has no symlink component injection.
            if !target_matches_source(&act.link_expanded, &act.source_canonical) {
                return StepResult::HardFailure(XdError::apply(format!(
                    "权限修复前重新校验失败: {} 不再指向 {}",
                    act.link_expanded.display(),
                    act.source_canonical.display()
                )));
            }
            if any_symlink_component(&act.source_canonical, &act.config_dir) {
                return StepResult::HardFailure(XdError::apply(format!(
                    "权限修复前重新校验失败: 源路径 {} 出现了符号链接组件",
                    act.source_canonical.display()
                )));
            }
            if !permissions::fix_permission(&act.source_canonical, *mode) {
                return StepResult::HardFailure(XdError::apply(format!(
                    "修复 {} 权限失败 ({} 要求 {:o})",
                    act.source_canonical.display(),
                    label,
                    mode
                )));
            }
            StepResult::Success
        }
        (PermissionAction::Fix, None) => StepResult::Success,
    }
}

fn ensure_parent_dir(link: &Path) -> Result<(), XdError> {
    if let Some(parent) = link.parent() {
        if !parent.exists() {
            fs::create_dir_all(parent).map_err(|e| {
                XdError::apply(format!("创建父目录失败 {}: {}", parent.display(), e))
            })?;
        }
    }
    Ok(())
}

fn recheck_link_missing(link: &Path) -> Result<(), XdError> {
    if fs::symlink_metadata(link).is_ok() {
        return Err(XdError::apply(format!(
            "应用阶段重新校验失败: {} 已存在",
            link.display()
        )));
    }
    Ok(())
}

fn recheck_existing_kind(link: &Path, expected: &ExistingKind) -> Result<(), XdError> {
    let meta = fs::symlink_metadata(link).map_err(|e| {
        XdError::apply(format!(
            "应用阶段重新校验失败: 无法读取 {}: {}",
            link.display(),
            e
        ))
    })?;
    let ft = meta.file_type();
    let actual = if ft.is_symlink() {
        match fs::read_link(link) {
            Ok(t) => {
                let abs = if t.is_absolute() {
                    t
                } else {
                    link.parent().map(|p| p.join(&t)).unwrap_or(t)
                };
                if abs.canonicalize().is_ok() {
                    ExistingKind::WrongSymlink
                } else {
                    ExistingKind::BrokenSymlink
                }
            }
            Err(_) => ExistingKind::BrokenSymlink,
        }
    } else if ft.is_file() {
        ExistingKind::RegularFile
    } else if ft.is_dir() {
        if fs::read_dir(link)
            .map(|mut it| it.next().is_none())
            .unwrap_or(false)
        {
            ExistingKind::EmptyRealDir
        } else {
            return Err(XdError::apply(format!(
                "应用阶段重新校验失败: {} 已变成非空目录",
                link.display()
            )));
        }
    } else {
        return Err(XdError::apply(format!(
            "应用阶段重新校验失败: {} 类型不再可处理",
            link.display()
        )));
    };
    if &actual != expected {
        return Err(XdError::apply(format!(
            "应用阶段重新校验失败: {} 类型变化 (规划时 {:?}, 当前 {:?})",
            link.display(),
            expected,
            actual
        )));
    }
    Ok(())
}

fn remove_existing(link: &Path, kind: &ExistingKind) -> Result<(), XdError> {
    match kind {
        ExistingKind::RegularFile | ExistingKind::WrongSymlink | ExistingKind::BrokenSymlink => {
            fs::remove_file(link)
                .map_err(|e| XdError::apply(format!("删除 {} 失败: {}", link.display(), e)))
        }
        ExistingKind::EmptyRealDir => fs::remove_dir(link)
            .map_err(|e| XdError::apply(format!("删除空目录 {} 失败: {}", link.display(), e))),
    }
}

fn target_matches_source(link: &Path, source_canon: &Path) -> bool {
    match fs::read_link(link) {
        Ok(t) => {
            let abs = if t.is_absolute() {
                t
            } else {
                link.parent().map(|p| p.join(&t)).unwrap_or(t)
            };
            abs.canonicalize()
                .map(|c| c == source_canon)
                .unwrap_or(false)
        }
        Err(_) => false,
    }
}

#[cfg(unix)]
fn create_symlink(link: &Path, source: &Path) -> io::Result<()> {
    unix_fs::symlink(source, link)
}

#[cfg(windows)]
fn create_symlink(link: &Path, source: &Path) -> io::Result<()> {
    if source.is_dir() {
        std::os::windows::fs::symlink_dir(source, link)
    } else {
        std::os::windows::fs::symlink_file(source, link)
    }
}

// -----------------------------------------------------------------------------
// Undeploy
// -----------------------------------------------------------------------------

pub fn apply_undeploy(plan: &UndeployPlan) -> ApplyOutcome {
    let mut out = ApplyOutcome::default();
    for act in &plan.actions {
        match apply_one_undeploy(act, plan.mode.interactive) {
            StepResult::Success => out.successes += 1,
            StepResult::SkippedFailure(e) => {
                out.failures += 1;
                out.errors.push(e);
                // continue with next link
            }
            StepResult::HardFailure(e) => {
                out.failures += 1;
                out.errors.push(e);
                break;
            }
        }
    }
    out
}

fn apply_one_undeploy(act: &UndeployAction, interactive: bool) -> StepResult {
    let link = &act.link_expanded;

    match &act.kind {
        UndeployActionKind::NotPresent => StepResult::Success,
        UndeployActionKind::NotASymlinkWarning => {
            // SPEC §undeploy table: "存在但不是符号链接 → 警告，计为失败，不删除"
            // — count as a failure but continue to the next link.
            eprintln!(
                "[警告] 链接路径 {} 是非符号链接对象，未删除",
                link.display()
            );
            StepResult::SkippedFailure(XdError::planning(format!(
                "链接路径 {} 不是符号链接，未删除",
                link.display()
            )))
        }
        UndeployActionKind::SkipFailure(reason) => StepResult::SkippedFailure(XdError::planning(
            format!("链接 {} 因可恢复冲突跳过: {}", link.display(), reason),
        )),
        UndeployActionKind::DeleteCorrect
        | UndeployActionKind::DeleteBroken
        | UndeployActionKind::DeleteWrong => {
            if interactive {
                let prompt = format!("删除 {}? [y/N] ", link.display());
                if !confirm(&prompt) {
                    return StepResult::SkippedFailure(XdError::planning(format!(
                        "链接 {} 在交互确认时被拒绝",
                        link.display()
                    )));
                }
            }
            // Apply-stage re-check: link must still be a symlink.
            match fs::symlink_metadata(link) {
                Err(_) => {
                    // It vanished — count as success per "link missing -> silent success".
                    return StepResult::Success;
                }
                Ok(m) => {
                    if !m.file_type().is_symlink() {
                        return StepResult::HardFailure(XdError::apply(format!(
                            "应用阶段重新校验失败: {} 不再是符号链接",
                            link.display()
                        )));
                    }
                }
            }
            if let Err(e) = fs::remove_file(link) {
                return StepResult::HardFailure(XdError::apply(format!(
                    "删除符号链接失败 {}: {}",
                    link.display(),
                    e
                )));
            }
            StepResult::Success
        }
    }
}

// -----------------------------------------------------------------------------
// Confirmation helper
// -----------------------------------------------------------------------------

/// Read a yes/no answer from stdin. Per SPEC: only `y` / `yes`
/// (case-insensitive) confirm; empty / EOF / non-TTY all reject.
pub fn confirm(prompt: &str) -> bool {
    if !io::stdin().is_terminal() {
        return false;
    }
    print!("{}", prompt);
    let _ = io::stdout().flush();
    let stdin = io::stdin();
    let mut line = String::new();
    let n = stdin.lock().read_line(&mut line).unwrap_or(0);
    if n == 0 {
        return false; // EOF
    }
    let trimmed = line.trim().to_ascii_lowercase();
    matches!(trimmed.as_str(), "y" | "yes")
}
