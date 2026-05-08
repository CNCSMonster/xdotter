use std::path::Path;

use crate::apply;
use crate::cli::{Cli, ConflictMode, DeployArgs};
use crate::discover;
use crate::error::{ErrorBag, XdError};
use crate::log;
use crate::plan::{
    self, DeployAction, DeployActionKind, DeployPlan, ExistingKind, PermissionAction,
};

pub fn run(cli: &Cli, args: &DeployArgs) -> Result<(), XdError> {
    let cwd = std::env::current_dir()
        .map_err(|e| XdError::cli(format!("无法获取当前工作目录: {}", e)))?;
    if !cwd.join("xdotter.toml").exists() {
        return Err(XdError::cli(format!(
            "当前目录 {} 中没有 xdotter.toml",
            cwd.display()
        )));
    }

    let mode = args.conflict_mode();
    log::info(
        cli,
        format!("deploy: 模式={:?}, dry_run={}", mode, args.dry_run),
    );
    let disc = discover::discover(&cwd);
    log::debug(
        cli,
        format!("deploy: 发现 {} 个配置文件", disc.configs.len()),
    );
    let res = plan::build_deploy_plan(disc, mode);

    if !res.errors.is_empty() {
        return Err(report_bag(res.errors));
    }

    log::debug(
        cli,
        format!("deploy: 规划 {} 条动作", res.plan.actions.len()),
    );
    if cli.verbose >= 1 {
        for a in &res.plan.actions {
            log_action(cli, a);
        }
    }

    if args.dry_run {
        print_deploy_plan(&res.plan, mode);
        if dry_run_plan_has_failures(&res.plan, mode) {
            return Err(XdError::planning(
                "dry-run 计划包含会跳过并计为失败的链接".to_string(),
            ));
        }
        return Ok(());
    }

    let outcome = apply::apply_deploy(&res.plan);
    print_deploy_outcome(&outcome, &res.plan);

    if outcome.failures > 0 || !outcome.errors.is_empty() {
        return Err(report_bag(outcome.errors));
    }
    Ok(())
}

fn log_action(cli: &Cli, a: &DeployAction) {
    let summary = match &a.kind {
        DeployActionKind::Create => "create",
        DeployActionKind::AlreadyCorrect => "already-correct",
        DeployActionKind::Replace(_) => "replace",
        DeployActionKind::SkipFailure(_) => "skip",
    };
    log::info(
        cli,
        format!(
            "  {} {} -> {}",
            summary,
            a.link_expanded.display(),
            a.source_canonical.display()
        ),
    );
}

fn report_bag(bag: ErrorBag) -> XdError {
    bag.into_single()
        .unwrap_or_else(|| XdError::apply("未知错误".to_string()))
}

fn dry_run_plan_has_failures(plan: &DeployPlan, mode: ConflictMode) -> bool {
    let interactive_dry_run = matches!(mode, ConflictMode::Interactive);
    plan.actions.iter().any(|a| {
        matches!(a.kind, DeployActionKind::SkipFailure(_))
            || (interactive_dry_run && matches!(a.kind, DeployActionKind::Replace(_)))
            || (interactive_dry_run && matches!(a.permission_action, PermissionAction::Fix))
            || matches!(a.permission_action, PermissionAction::SkipFailure(_))
    })
}

fn print_deploy_plan(plan: &DeployPlan, mode: ConflictMode) {
    // SPEC §"执行模式": `--interactive --dry-run` is treated as "no" for
    // every recoverable conflict; `--force --dry-run` is treated as
    // "yes". This affects how Replace and PermissionAction::Fix are
    // rendered in the dry-run output — `apply` is never called here.
    let interactive_dry_run = matches!(mode, ConflictMode::Interactive);

    println!("# Deploy plan ({} 条目)", plan.actions.len());
    for a in &plan.actions {
        // Sensitive-target advisory belongs on stderr (warnings / diagnostics).
        if let Some((m, label)) = a.permission_required {
            eprintln!(
                "[警告] 链接 {} 命中敏感目标 ({}, 期望权限 {:o})；请确认该路径由 xdotter 管理",
                a.link_expanded.display(),
                label,
                m
            );
        }

        let (marker, desc) = describe_action_for_dry_run(&a.kind, interactive_dry_run);
        println!(
            "{} {} -> {} [{}]",
            marker,
            display_link(&a.link_expanded),
            a.source_canonical.display(),
            desc
        );
        if let (Some((mode_required, label)), action) =
            (a.permission_required, &a.permission_action)
        {
            match action {
                PermissionAction::None => {}
                PermissionAction::AlreadyOk => {
                    println!("    perm: ok ({}, {:o})", label, mode_required);
                }
                PermissionAction::Fix => {
                    if interactive_dry_run {
                        println!("    perm: would skip permission fix (interactive declined)");
                    } else {
                        println!("    perm: would fix to {:o} ({})", mode_required, label);
                    }
                }
                PermissionAction::SkipFailure(r) => {
                    println!("    perm: skip ({})", r);
                }
            }
        }
    }
}

/// Render a deploy action for dry-run, accounting for interactive mode
/// (every recoverable conflict is treated as "no" per SPEC).
fn describe_action_for_dry_run(
    k: &DeployActionKind,
    interactive_dry_run: bool,
) -> (&'static str, String) {
    match k {
        DeployActionKind::Create => ("+", "create".to_string()),
        DeployActionKind::AlreadyCorrect => ("=", "already correct".to_string()),
        DeployActionKind::Replace(existing) => {
            if interactive_dry_run {
                (
                    "!",
                    format!(
                        "would skip {} (interactive declined)",
                        describe_existing(existing)
                    ),
                )
            } else {
                ("~", format!("replace {}", describe_existing(existing)))
            }
        }
        DeployActionKind::SkipFailure(r) => ("!", format!("skip: {}", r)),
    }
}

fn print_deploy_outcome(outcome: &apply::ApplyOutcome, plan: &DeployPlan) {
    eprintln!(
        "Deploy: {} succeeded, {} skipped, {} failed (planned {})",
        outcome.successes,
        outcome.skipped,
        outcome.failures,
        plan.actions.len()
    );
    for e in outcome.errors.iter() {
        eprintln!("{}", e);
    }
}

fn describe_existing(e: &ExistingKind) -> &'static str {
    match e {
        ExistingKind::RegularFile => "regular file",
        ExistingKind::EmptyRealDir => "empty dir",
        ExistingKind::WrongSymlink => "wrong symlink",
        ExistingKind::BrokenSymlink => "broken symlink",
    }
}

fn display_link(p: &Path) -> String {
    p.display().to_string()
}
