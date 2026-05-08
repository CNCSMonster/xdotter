use crate::cli::Cli;
use crate::discover;
use crate::error::XdError;
use crate::plan::{self, LinkStatus, LinkStatusRecord};

pub fn run(cli: &Cli) -> Result<(), XdError> {
    let cwd = std::env::current_dir()
        .map_err(|e| XdError::cli(format!("无法获取当前工作目录: {}", e)))?;
    if !cwd.join("xdotter.toml").exists() {
        return Err(XdError::cli(format!(
            "当前目录 {} 中没有 xdotter.toml",
            cwd.display()
        )));
    }

    let disc = discover::discover(&cwd);
    let result = plan::build_status(disc);

    if !result.errors.is_empty() {
        for e in result.errors.iter() {
            eprintln!("{}", e);
        }
        return Err(result
            .errors
            .into_single()
            .unwrap_or_else(|| XdError::config("status: 配置错误".to_string())));
    }

    let total = result.records.len();
    let mut deployed = 0usize;
    let mut not_deployed = 0usize;
    let mut wrong = 0usize;
    let mut broken = 0usize;
    let mut source_missing = 0usize;
    let mut source_type_invalid = 0usize;
    let mut non_symlink = 0usize;
    let mut perm = 0usize;

    let verbose = cli.verbose >= 1;

    for r in &result.records {
        if verbose || !matches!(r.status, LinkStatus::Deployed) || r.permission_issue.is_some() {
            print_record(r);
        }
        match r.status {
            LinkStatus::Deployed => deployed += 1,
            LinkStatus::NotDeployed => not_deployed += 1,
            LinkStatus::WrongLink => wrong += 1,
            LinkStatus::BrokenLink => broken += 1,
            LinkStatus::SourceMissing => source_missing += 1,
            LinkStatus::SourceTypeInvalid => source_type_invalid += 1,
            LinkStatus::NonSymlink => non_symlink += 1,
        }
        if r.permission_issue.is_some() {
            perm += 1;
        }
    }

    // SPEC fixed-format summary, exactly seven lines + the Status line.
    println!("Status: {}/{} deployed", deployed, total);
    println!("Not deployed: {}", not_deployed);
    println!("Wrong links: {}", wrong);
    println!("Broken links: {}", broken);
    println!("Source missing: {}", source_missing);
    println!("Source type invalid: {}", source_type_invalid);
    println!("Non-symlink paths: {}", non_symlink);
    println!("Permission issues: {}", perm);

    let any_problem =
        not_deployed + wrong + broken + source_missing + source_type_invalid + non_symlink + perm
            > 0;
    if any_problem {
        eprintln!("status: 存在未部署/错误/损坏链接、源问题、非符号链接对象或权限问题");
        std::process::exit(1);
    }
    Ok(())
}

fn print_record(r: &LinkStatusRecord) {
    let label = match r.status {
        LinkStatus::Deployed => "deployed",
        LinkStatus::NotDeployed => "not-deployed",
        LinkStatus::WrongLink => "wrong-link",
        LinkStatus::BrokenLink => "broken-link",
        LinkStatus::SourceMissing => "source-missing",
        LinkStatus::SourceTypeInvalid => "source-type-invalid",
        LinkStatus::NonSymlink => "non-symlink",
    };
    let perm = match r.permission_issue {
        Some((m, lbl)) => format!("  permission-issue ({} 要求 {:o})", lbl, m),
        None => String::new(),
    };
    println!(
        "[{}] {} -> {} (源 \"{}\"){}",
        label,
        r.link_expanded.display(),
        r.config_file.display(),
        r.source_raw,
        perm,
    );
}
