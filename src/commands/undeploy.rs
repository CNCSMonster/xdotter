use crate::apply;
use crate::cli::{Cli, ConflictMode, UndeployArgs};
use crate::discover;
use crate::error::{ErrorBag, XdError};
use crate::log;
use crate::plan::{self, UndeployAction, UndeployActionKind, UndeployPlan};

pub fn run(cli: &Cli, args: &UndeployArgs) -> Result<(), XdError> {
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
        format!("undeploy: 模式={:?}, dry_run={}", mode, args.dry_run),
    );
    let disc = discover::discover(&cwd);
    log::debug(
        cli,
        format!("undeploy: 发现 {} 个配置文件", disc.configs.len()),
    );
    let res = plan::build_undeploy_plan(disc, mode);

    if !res.errors.is_empty() {
        return Err(report_bag(res.errors));
    }

    log::debug(
        cli,
        format!("undeploy: 规划 {} 条动作", res.plan.actions.len()),
    );
    if cli.verbose >= 1 {
        for a in &res.plan.actions {
            log_action(cli, a);
        }
    }

    if args.dry_run {
        print_undeploy_plan(&res.plan, mode);
        if dry_run_plan_has_failures(&res.plan, mode) {
            return Err(XdError::planning(
                "dry-run 计划包含会跳过并计为失败的链接".to_string(),
            ));
        }
        return Ok(());
    }

    let outcome = apply::apply_undeploy(&res.plan);
    print_undeploy_outcome(&outcome, &res.plan);

    if outcome.failures > 0 || !outcome.errors.is_empty() {
        return Err(report_bag(outcome.errors));
    }
    Ok(())
}

fn log_action(cli: &Cli, a: &UndeployAction) {
    let summary = match &a.kind {
        UndeployActionKind::NotPresent => "absent",
        UndeployActionKind::DeleteCorrect => "delete-correct",
        UndeployActionKind::DeleteBroken => "delete-broken",
        UndeployActionKind::DeleteWrong => "delete-wrong",
        UndeployActionKind::SkipFailure(_) => "skip",
        UndeployActionKind::NotASymlinkWarning => "warn-not-symlink",
    };
    log::info(cli, format!("  {} {}", summary, a.link_expanded.display()));
}

fn report_bag(bag: ErrorBag) -> XdError {
    bag.into_single()
        .unwrap_or_else(|| XdError::apply("未知错误".to_string()))
}

fn dry_run_plan_has_failures(plan: &UndeployPlan, mode: ConflictMode) -> bool {
    let interactive_dry_run = matches!(mode, ConflictMode::Interactive);
    plan.actions.iter().any(|a| {
        matches!(
            a.kind,
            UndeployActionKind::SkipFailure(_) | UndeployActionKind::NotASymlinkWarning
        ) || (interactive_dry_run
            && matches!(
                a.kind,
                UndeployActionKind::DeleteCorrect
                    | UndeployActionKind::DeleteBroken
                    | UndeployActionKind::DeleteWrong
            ))
    })
}

fn print_undeploy_plan(plan: &UndeployPlan, mode: ConflictMode) {
    // SPEC §"执行模式": `--interactive --dry-run` is treated as "no" for
    // every recoverable conflict; `--force --dry-run` is treated as
    // "yes". Rendering reflects that — `apply` is never called here.
    let interactive_dry_run = matches!(mode, ConflictMode::Interactive);

    println!("# Undeploy plan ({} 条目)", plan.actions.len());
    for a in &plan.actions {
        let (marker, desc) = render_action(&a.kind, interactive_dry_run);
        println!("{} {} [{}]", marker, a.link_expanded.display(), desc);
    }
}

fn render_action(k: &UndeployActionKind, interactive_dry_run: bool) -> (&'static str, String) {
    match k {
        UndeployActionKind::NotPresent => ("·", "absent (silent success)".to_string()),
        UndeployActionKind::DeleteCorrect => render_delete("correct symlink", interactive_dry_run),
        UndeployActionKind::DeleteBroken => render_delete("broken symlink", interactive_dry_run),
        UndeployActionKind::DeleteWrong => render_delete("wrong symlink", interactive_dry_run),
        UndeployActionKind::SkipFailure(r) => ("!", format!("skip: {}", r)),
        UndeployActionKind::NotASymlinkWarning => ("!", "warning: not a symlink".to_string()),
    }
}

fn render_delete(kind: &str, interactive_dry_run: bool) -> (&'static str, String) {
    if interactive_dry_run {
        (
            "!",
            format!("would skip delete {} (interactive declined)", kind),
        )
    } else {
        ("-", format!("delete {}", kind))
    }
}

fn print_undeploy_outcome(outcome: &apply::ApplyOutcome, plan: &UndeployPlan) {
    eprintln!(
        "Undeploy: {} succeeded, {} skipped, {} failed (planned {})",
        outcome.successes,
        outcome.skipped,
        outcome.failures,
        plan.actions.len()
    );
    for e in outcome.errors.iter() {
        eprintln!("{}", e);
    }
}
