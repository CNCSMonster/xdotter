//! Planning stage per SPEC §"规划阶段".
//!
//! Consumes the result of `discover` and produces a [`DeployPlan`] /
//! [`UndeployPlan`]. All configuration errors and planning-block errors
//! across the root and all reachable dependencies are collected before
//! returning so the user sees them in one shot.
//!
//! No filesystem modification happens here.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::cli::ConflictMode;
use crate::discover::{is_inside, Discovered, DiscoveredConfig};
use crate::error::{ErrorBag, XdError};
use crate::path as p;
use crate::permissions;

// -----------------------------------------------------------------------------
// Plan data types
// -----------------------------------------------------------------------------

/// One planned action against a single link path.
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct DeployAction {
    /// Origin config file (for diagnostics).
    pub config_file: PathBuf,
    /// Config directory (used for apply-stage re-checks).
    pub config_dir: PathBuf,
    /// Raw source-path string from ``[links]`` key.
    pub source_raw: String,
    /// Canonicalized source path on disk.
    pub source_canonical: PathBuf,
    /// Raw link-path string from ``[links]`` value.
    pub link_raw: String,
    /// ``~/``-expanded link path (lexical, not canonical).
    pub link_expanded: PathBuf,
    /// What we plan to do at the link path.
    pub kind: DeployActionKind,
    /// True iff this link path matches a SPEC permission target.
    /// In that case ``permission_required`` is ``Some((mode, label))`` and
    /// ``permission_action`` describes what we'll do about it.
    pub permission_required: Option<(u32, &'static str)>,
    pub permission_action: PermissionAction,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeployActionKind {
    /// Link path does not exist; create the symlink.
    Create,
    /// Link path is already a correct symlink to source; skip.
    AlreadyCorrect,
    /// Link path holds something we need to replace (regular file,
    /// wrong/broken symlink, or empty real directory). The contained
    /// `Replace` describes the existing object.
    Replace(ExistingKind),
    /// Recoverable conflict the current mode cannot handle; skip the
    /// link and count as failure.
    SkipFailure(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExistingKind {
    RegularFile,
    EmptyRealDir,
    WrongSymlink,
    BrokenSymlink,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PermissionAction {
    None,
    AlreadyOk,
    Fix,
    SkipFailure(String),
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct UndeployAction {
    pub config_file: PathBuf,
    pub source_raw: String,
    pub source_canonical: Option<PathBuf>,
    pub link_raw: String,
    pub link_expanded: PathBuf,
    pub kind: UndeployActionKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UndeployActionKind {
    /// Link path doesn't exist — silent success.
    NotPresent,
    /// Symlink whose target equals the configured source — delete.
    DeleteCorrect,
    /// Symlink whose target does not exist — delete.
    DeleteBroken,
    /// Symlink with the wrong target — recoverable conflict.
    DeleteWrong,
    /// Skip and count as failure (e.g. wrong-symlink in default mode).
    SkipFailure(String),
    /// Existing non-symlink (regular file/dir) at the link path —
    /// warning, count as failure, do not delete.
    NotASymlinkWarning,
}

#[derive(Debug, Default)]
pub struct DeployPlan {
    pub actions: Vec<DeployAction>,
    pub mode: ConflictModeRecord,
}

#[derive(Debug, Default)]
pub struct UndeployPlan {
    pub actions: Vec<UndeployAction>,
    pub mode: ConflictModeRecord,
}

#[allow(dead_code)]
#[derive(Debug, Default, Clone, Copy)]
pub struct ConflictModeRecord {
    pub force: bool,
    pub interactive: bool,
}

impl ConflictModeRecord {
    pub fn from(m: ConflictMode) -> Self {
        Self {
            force: matches!(m, ConflictMode::Force),
            interactive: matches!(m, ConflictMode::Interactive),
        }
    }
}

// -----------------------------------------------------------------------------
// Status data type for `xd status`
// -----------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LinkStatus {
    Deployed,
    NotDeployed,
    WrongLink,
    BrokenLink,
    SourceMissing,
    SourceTypeInvalid,
    NonSymlink,
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct LinkStatusRecord {
    pub config_file: PathBuf,
    pub source_raw: String,
    pub link_raw: String,
    pub link_expanded: PathBuf,
    pub status: LinkStatus,
    pub permission_issue: Option<(u32, &'static str)>,
}

// -----------------------------------------------------------------------------
// Build plans
// -----------------------------------------------------------------------------

pub struct DeployPlanResult {
    pub plan: DeployPlan,
    pub errors: ErrorBag,
}

pub struct UndeployPlanResult {
    pub plan: UndeployPlan,
    pub errors: ErrorBag,
}

pub struct StatusResult {
    pub records: Vec<LinkStatusRecord>,
    pub errors: ErrorBag,
}

/// Construct a deploy plan from a successful discovery. Errors from
/// `disc.errors` are forwarded verbatim; new errors from this stage are
/// appended.
pub fn build_deploy_plan(disc: Discovered, mode: ConflictMode) -> DeployPlanResult {
    let mut errors = disc.errors;
    let entries = collect_global_links(&disc.configs, &mut errors);

    let mut actions = Vec::new();
    for ge in entries {
        match plan_one_deploy(&ge, mode) {
            Ok(Some(act)) => actions.push(act),
            Ok(None) => {}
            Err(e) => errors.push(e),
        }
    }

    DeployPlanResult {
        plan: DeployPlan {
            actions,
            mode: ConflictModeRecord::from(mode),
        },
        errors,
    }
}

pub fn build_undeploy_plan(disc: Discovered, mode: ConflictMode) -> UndeployPlanResult {
    let mut errors = disc.errors;
    let entries = collect_global_links(&disc.configs, &mut errors);

    let mut actions = Vec::new();
    for ge in entries {
        match plan_one_undeploy(&ge, mode) {
            Ok(Some(act)) => actions.push(act),
            Ok(None) => {}
            Err(e) => errors.push(e),
        }
    }

    UndeployPlanResult {
        plan: UndeployPlan {
            actions,
            mode: ConflictModeRecord::from(mode),
        },
        errors,
    }
}

/// Build a status report. Status does not need conflict modes; it just
/// classifies each link.
pub fn build_status(disc: Discovered) -> StatusResult {
    let mut errors = disc.errors;
    let entries = collect_global_links(&disc.configs, &mut errors);

    let mut records = Vec::new();
    for ge in entries {
        records.push(classify_link_for_status(&ge));
    }
    StatusResult { records, errors }
}

// -----------------------------------------------------------------------------
// Internal: global link collection + per-entry resolution
// -----------------------------------------------------------------------------

#[allow(dead_code)]
#[derive(Debug, Clone)]
struct GlobalEntry {
    config_file: PathBuf,
    config_dir: PathBuf,
    source_raw: String,
    /// Source resolved against config_dir, lexically (no symlink resolution).
    source_resolved: PathBuf,
    link_raw: String,
    /// `~/`-expanded, lexically normalized link path (used for global
    /// uniqueness comparison).
    link_expanded: PathBuf,
    /// Same as `link_expanded` but canonical-normalized for stable
    /// dedup keys (no filesystem canonicalize).
    link_key: PathBuf,
}

fn collect_global_links(configs: &[DiscoveredConfig], errors: &mut ErrorBag) -> Vec<GlobalEntry> {
    // Per-config: validate static rules on raw source/link strings.
    // Global: detect link-path collisions and report ALL conflicting
    // declarations.
    let mut entries: Vec<GlobalEntry> = Vec::new();
    let mut by_link: BTreeMap<PathBuf, Vec<usize>> = BTreeMap::new();

    for c in configs {
        for (src_raw, link_raw) in &c.config.links {
            // Static source-path rules.
            if let Err(e) = p::validate_source_path(src_raw) {
                errors.push(decorate(&e, &c.config_file));
                continue;
            }
            // Static link-path rules.
            if let Err(e) = p::validate_link_path(link_raw) {
                errors.push(decorate(&e, &c.config_file));
                continue;
            }
            let source_resolved = p::normalize(&c.config_dir.join(src_raw));
            let expanded = match p::expand_tilde(link_raw) {
                Ok(p) => p::normalize(&p),
                Err(e) => {
                    errors.push(decorate(&e, &c.config_file));
                    continue;
                }
            };
            let key = expanded.clone();
            let idx = entries.len();
            entries.push(GlobalEntry {
                config_file: c.config_file.clone(),
                config_dir: c.config_dir.clone(),
                source_raw: src_raw.clone(),
                source_resolved,
                link_raw: link_raw.clone(),
                link_expanded: expanded,
                link_key: key.clone(),
            });
            by_link.entry(key).or_default().push(idx);
        }
    }

    // Detect collisions and emit one config error per collision group,
    // listing all (config_file, source) pairs.
    let mut bad_indices: std::collections::BTreeSet<usize> = Default::default();
    for (link_key, idxs) in &by_link {
        if idxs.len() < 2 {
            continue;
        }
        let mut listing = String::new();
        for &i in idxs {
            let e = &entries[i];
            listing.push_str(&format!(
                "\n  - {} (源 \"{}\")",
                e.config_file.display(),
                e.source_raw
            ));
            bad_indices.insert(i);
        }
        errors.push(XdError::config(format!(
            "多个链接条目展开后指向同一链接路径 {}：{}",
            link_key.display(),
            listing
        )));
    }

    // Detect topological nesting: no link path may be inside another link
    // path, because the parent link would be a symlink and creating a
    // child inside it is unsafe (SPEC §"符号链接安全语义").
    let nesting_pairs = detect_link_nesting(&entries);
    for (outer_idx, inner_idx) in nesting_pairs {
        let outer = &entries[outer_idx];
        let inner = &entries[inner_idx];
        errors.push(XdError::planning(format!(
            "链接路径 {} 位于另一链接路径 {} 内部（源 \"{}\"），不允许在符号链接内部创建子链接",
            inner.link_expanded.display(),
            outer.link_expanded.display(),
            inner.source_raw
        )));
        bad_indices.insert(outer_idx);
        bad_indices.insert(inner_idx);
    }
    // Drop the conflicting entries from the planning stream so we don't
    // try to deploy or undeploy them.
    entries
        .into_iter()
        .enumerate()
        .filter_map(|(i, e)| {
            if bad_indices.contains(&i) {
                None
            } else {
                Some(e)
            }
        })
        .collect()
}

/// Detect pairs of link entries where one expanded link path is inside
fn detect_link_nesting(entries: &[GlobalEntry]) -> Vec<(usize, usize)> {
    let mut pairs = Vec::new();
    for i in 0..entries.len() {
        for j in 0..entries.len() {
            if i == j {
                continue;
            }
            let outer = &entries[i].link_expanded;
            let inner = &entries[j].link_expanded;
            if is_proper_path_prefix(outer, inner) {
                pairs.push((i, j));
            }
        }
    }
    pairs
}

/// Check if `outer` is a proper path prefix of `inner` — i.e.,
/// `inner` starts with all components of `outer` and has additional
/// components beyond that. This avoids `starts_with` false positives
/// like `/home/user/.a` matching `/home/user/.abc`.
fn is_proper_path_prefix(outer: &Path, inner: &Path) -> bool {
    let mut outer_comps = outer.components();
    let mut inner_comps = inner.components();
    loop {
        match (outer_comps.next(), inner_comps.next()) {
            (Some(o), Some(i)) if o == i => continue,
            (None, Some(_)) => return true,
            _ => return false,
        }
    }
}

fn decorate(e: &XdError, toml: &Path) -> XdError {
    let msg = format!("{}: {}", toml.display(), e.body());
    match e {
        XdError::Cli(_) => XdError::Cli(msg),
        XdError::Config(_) => XdError::Config(msg),
        XdError::Planning(_) => XdError::Planning(msg),
        XdError::Apply(_) => XdError::Apply(msg),
    }
}

// -----------------------------------------------------------------------------
// Per-entry deploy planning
// -----------------------------------------------------------------------------
fn plan_one_deploy(ge: &GlobalEntry, mode: ConflictMode) -> Result<Option<DeployAction>, XdError> {
    // 1. Source must exist and be a regular file or directory; no
    //    component (final or intermediate) may be a symlink; must stay
    //    inside config dir tree.
    let source_canonical = match validate_source_filesystem(&ge.source_resolved, &ge.config_dir) {
        Ok(p) => p,
        Err(e) => return Err(decorate(&e, &ge.config_file)),
    };

    // 2. Inspect existing object at link_expanded before topology checks.
    //    A correct existing symlink is already deployed and must not be
    //    rejected merely because following the symlink reaches the source.
    let kind = classify_link_target(&ge.link_expanded, &source_canonical);

    // 3. Topological safety with the link path. Already-correct symlinks
    //    do not require creating/replacing the link itself, so parent/link
    //    creation topology is irrelevant for that action (permission checks
    //    are handled separately below).
    if !matches!(kind, LinkSlot::CorrectSymlink) {
        if let Err(e) = check_topology(&ge.link_expanded, &source_canonical) {
            return Err(decorate(&e, &ge.config_file));
        }
    }

    // Build action_kind based on mode.
    let mut action_kind = match kind {
        LinkSlot::Missing => DeployActionKind::Create,
        LinkSlot::CorrectSymlink => DeployActionKind::AlreadyCorrect,
        LinkSlot::WrongSymlink => act_for_replace(ExistingKind::WrongSymlink, mode),
        LinkSlot::BrokenSymlink => act_for_replace(ExistingKind::BrokenSymlink, mode),
        LinkSlot::RegularFile => act_for_replace(ExistingKind::RegularFile, mode),
        LinkSlot::EmptyRealDir => {
            // Replacing this directory must not delete or contain the source.
            if dir_contains_source(&ge.link_expanded, &source_canonical) {
                return Err(decorate(
                    &XdError::planning(format!(
                        "目标目录 {} 包含源路径 {}，替换会删除或包含源",
                        ge.link_expanded.display(),
                        source_canonical.display()
                    )),
                    &ge.config_file,
                ));
            }
            act_for_replace(ExistingKind::EmptyRealDir, mode)
        }
        LinkSlot::NonEmptyRealDir => DeployActionKind::SkipFailure(
            "目标是非空真实目录，xdotter 不递归删除真实目录".to_string(),
        ),
    };

    // 4. Permissions (only if link path matches SPEC table). A default-mode
    //    permission issue is a recoverable conflict that skips the entire
    //    link before any create/replace filesystem modification happens.
    let (perm_required, perm_action) = plan_permission(&ge.link_expanded, &source_canonical, mode);
    if let PermissionAction::SkipFailure(reason) = &perm_action {
        action_kind = DeployActionKind::SkipFailure(reason.clone());
    }

    Ok(Some(DeployAction {
        config_file: ge.config_file.clone(),
        config_dir: ge.config_dir.clone(),
        source_raw: ge.source_raw.clone(),
        source_canonical,
        link_raw: ge.link_raw.clone(),
        link_expanded: ge.link_expanded.clone(),
        kind: action_kind,
        permission_required: perm_required,
        permission_action: perm_action,
    }))
}

fn act_for_replace(existing: ExistingKind, mode: ConflictMode) -> DeployActionKind {
    match mode {
        ConflictMode::Default => DeployActionKind::SkipFailure(format!(
            "默认模式不替换已有对象 ({})",
            describe_existing(&existing)
        )),
        ConflictMode::Force | ConflictMode::Interactive => DeployActionKind::Replace(existing),
    }
}

fn describe_existing(k: &ExistingKind) -> &'static str {
    match k {
        ExistingKind::RegularFile => "普通文件",
        ExistingKind::EmptyRealDir => "空真实目录",
        ExistingKind::WrongSymlink => "错误符号链接",
        ExistingKind::BrokenSymlink => "损坏符号链接",
    }
}

fn plan_permission(
    link_expanded: &Path,
    source_canonical: &Path,
    mode: ConflictMode,
) -> (Option<(u32, &'static str)>, PermissionAction) {
    let key = match link_path_to_tilde_key(link_expanded) {
        Some(k) => k,
        None => return (None, PermissionAction::None),
    };
    let (mode_required, label) = match permissions::required_permission(&key) {
        Some(v) => v,
        None => return (None, PermissionAction::None),
    };
    if permissions::check_permission(source_canonical, mode_required) {
        return (Some((mode_required, label)), PermissionAction::AlreadyOk);
    }
    let action = match mode {
        ConflictMode::Default => PermissionAction::SkipFailure(format!(
            "{} 权限过宽（要求不宽于 {:o}）",
            label, mode_required
        )),
        ConflictMode::Force | ConflictMode::Interactive => PermissionAction::Fix,
    };
    (Some((mode_required, label)), action)
}

/// Convert an absolute, expanded link path to the `~/...` form used by
/// the SPEC permission table. Returns `None` if the path is not under
/// `$HOME`.
pub fn link_path_to_tilde_key(link: &Path) -> Option<String> {
    let home = home_dir()?;
    let stripped = link.strip_prefix(&home).ok()?;
    if stripped.as_os_str().is_empty() {
        return Some("~".to_string());
    }
    Some(format!("~/{}", stripped.display()))
}

fn home_dir() -> Option<PathBuf> {
    if let Ok(h) = std::env::var("HOME") {
        if !h.is_empty() {
            return Some(PathBuf::from(h));
        }
    }
    dirs::home_dir()
}

// -----------------------------------------------------------------------------
// Per-entry undeploy planning
// -----------------------------------------------------------------------------

fn plan_one_undeploy(
    ge: &GlobalEntry,
    mode: ConflictMode,
) -> Result<Option<UndeployAction>, XdError> {
    // SPEC: undeploy applies the same dependency-path rules as deploy
    // (already covered by discover) and the same link-path rules
    // (already covered by collect_global_links). Source need not exist
    // for undeploy to function, but we still resolve it to compare
    // symlink targets.
    let source_canonical = ge.source_resolved.canonicalize().ok();

    let kind = match read_link_target(&ge.link_expanded) {
        LinkProbe::DoesNotExist => UndeployActionKind::NotPresent,
        LinkProbe::NotASymlink => UndeployActionKind::NotASymlinkWarning,
        LinkProbe::Symlink {
            target_exists,
            target_abs,
            target_canonical,
        } => {
            // Does it point to *our* configured source?
            let canonically_ours = match (&source_canonical, &target_canonical) {
                (Some(s), Some(t)) => s == t,
                _ => false,
            };
            // Textual match: when canonicalize fails (e.g. source
            // temporarily unavailable), the lexical target path
            // may still match our configured source.
            let textually_ours = match &source_canonical {
                Some(s) => &target_abs == s,
                None => false,
            };
            let is_ours = canonically_ours || textually_ours;
            if is_ours {
                UndeployActionKind::DeleteCorrect
            } else if !target_exists {
                UndeployActionKind::DeleteBroken
            } else {
                // Wrong symlink — recoverable conflict.
                match mode {
                    ConflictMode::Default => {
                        UndeployActionKind::SkipFailure("默认模式不删除错误符号链接".to_string())
                    }
                    ConflictMode::Force | ConflictMode::Interactive => {
                        UndeployActionKind::DeleteWrong
                    }
                }
            }
        }
    };

    Ok(Some(UndeployAction {
        config_file: ge.config_file.clone(),
        source_raw: ge.source_raw.clone(),
        source_canonical,
        link_raw: ge.link_raw.clone(),
        link_expanded: ge.link_expanded.clone(),
        kind,
    }))
}

// -----------------------------------------------------------------------------
// Per-entry status classification
// -----------------------------------------------------------------------------

fn classify_link_for_status(ge: &GlobalEntry) -> LinkStatusRecord {
    let lp = &ge.link_expanded;
    let mut status = LinkStatus::NotDeployed;
    let mut permission_issue = None;

    let exists_or_link = lp.exists() || lp.is_symlink();
    if !exists_or_link {
        // not deployed
    } else if !lp.is_symlink() {
        status = LinkStatus::NonSymlink;
    } else {
        // It is a symlink — read its target.
        match fs::read_link(lp) {
            Ok(t) => {
                let target_abs = if t.is_absolute() {
                    t.clone()
                } else {
                    lp.parent().map(|pp| pp.join(&t)).unwrap_or(t.clone())
                };
                let target_meta = fs::metadata(&target_abs);
                if target_meta.is_err() {
                    status = LinkStatus::BrokenLink;
                } else {
                    // Compare canonical of target vs canonical of configured source.
                    let target_canon = target_abs.canonicalize().ok();
                    let source_canon = ge.source_resolved.canonicalize().ok();
                    let points_to_us = match (&target_canon, &source_canon) {
                        (Some(a), Some(b)) => a == b,
                        _ => false,
                    };
                    if !points_to_us {
                        status = LinkStatus::WrongLink;
                    } else {
                        // Now check source health for SPEC §"源路径".
                        if !ge.source_resolved.exists() {
                            status = LinkStatus::SourceMissing;
                        } else if !is_regular_file_or_dir(&ge.source_resolved)
                            || any_symlink_component(&ge.source_resolved, &ge.config_dir)
                        {
                            status = LinkStatus::SourceTypeInvalid;
                        } else {
                            status = LinkStatus::Deployed;
                        }
                    }
                }
            }
            Err(_) => status = LinkStatus::BrokenLink,
        }
    }

    // Permission issue (SPEC: independent of deployment status).
    if let Some(key) = link_path_to_tilde_key(lp) {
        if let Some((mode, label)) = permissions::required_permission(&key) {
            // Only meaningful when source object exists.
            let src = ge.source_resolved.canonicalize().ok();
            if let Some(src) = src {
                if !permissions::check_permission(&src, mode) {
                    permission_issue = Some((mode, label));
                }
            }
        }
    }

    LinkStatusRecord {
        config_file: ge.config_file.clone(),
        source_raw: ge.source_raw.clone(),
        link_raw: ge.link_raw.clone(),
        link_expanded: ge.link_expanded.clone(),
        status,
        permission_issue,
    }
}

// -----------------------------------------------------------------------------
// Filesystem-state probes
// -----------------------------------------------------------------------------

/// Validate the source path against the filesystem per SPEC.
fn validate_source_filesystem(source: &Path, config_dir: &Path) -> Result<PathBuf, XdError> {
    if !source.exists() && !source.is_symlink() {
        return Err(XdError::planning(format!(
            "源路径不存在: {}",
            source.display()
        )));
    }
    if any_symlink_component(source, config_dir) {
        return Err(XdError::planning(format!(
            "源路径任一组件是符号链接: {}",
            source.display()
        )));
    }
    let canon = source
        .canonicalize()
        .map_err(|e| XdError::planning(format!("无法访问源路径 {}: {}", source.display(), e)))?;
    if !is_regular_file_or_dir(&canon) {
        return Err(XdError::planning(format!(
            "源路径不是普通文件或目录: {}",
            source.display()
        )));
    }
    let canon_dir = config_dir
        .canonicalize()
        .unwrap_or_else(|_| config_dir.to_path_buf());
    if !is_inside(&canon, &canon_dir) {
        return Err(XdError::planning(format!(
            "源路径解析后逃出当前配置目录树: {}",
            source.display()
        )));
    }
    Ok(canon)
}

/// True iff any component of `p`, walked from `boundary` downward, is
/// a symlink. `boundary` is the config dir; everything at or above it
/// is not the source's responsibility.
pub(crate) fn any_symlink_component(p: &Path, boundary: &Path) -> bool {
    let boundary = boundary
        .canonicalize()
        .unwrap_or_else(|_| boundary.to_path_buf());
    let mut cur = PathBuf::new();
    for comp in p.components() {
        cur.push(comp.as_os_str());
        // Skip components up to and including the boundary.
        let Ok(cmeta) = fs::symlink_metadata(&cur) else {
            // Component does not yet exist — we won't classify as symlink.
            continue;
        };
        if cmeta.file_type().is_symlink() {
            // If `cur` is inside or equal to boundary, ignore.
            if let Ok(canon_cur) = cur.canonicalize() {
                if !canon_cur.starts_with(&boundary) {
                    // doesn't matter for this check — what matters is "is symlink".
                }
            }
            // The boundary itself or its ancestors being symlinks is
            // outside of source-path scope; only report symlinks for
            // components strictly inside boundary.
            if let Ok(rel) = cur.strip_prefix(&boundary) {
                if !rel.as_os_str().is_empty() {
                    return true;
                }
            }
        }
    }
    false
}

fn is_regular_file_or_dir(canon: &Path) -> bool {
    match fs::metadata(canon) {
        Ok(m) => m.is_file() || m.is_dir(),
        Err(_) => false,
    }
}

#[derive(Debug)]
enum LinkSlot {
    Missing,
    CorrectSymlink,
    WrongSymlink,
    BrokenSymlink,
    RegularFile,
    EmptyRealDir,
    NonEmptyRealDir,
}

fn classify_link_target(link: &Path, source_canon: &Path) -> LinkSlot {
    let symlink_meta = fs::symlink_metadata(link);
    match symlink_meta {
        Err(_) => LinkSlot::Missing,
        Ok(m) => {
            let ft = m.file_type();
            if ft.is_symlink() {
                match fs::read_link(link) {
                    Ok(t) => {
                        let abs = if t.is_absolute() {
                            t
                        } else {
                            link.parent().map(|p| p.join(&t)).unwrap_or(t)
                        };
                        match abs.canonicalize() {
                            Ok(c) if c == source_canon => LinkSlot::CorrectSymlink,
                            Ok(_) => LinkSlot::WrongSymlink,
                            Err(_) => LinkSlot::BrokenSymlink,
                        }
                    }
                    Err(_) => LinkSlot::BrokenSymlink,
                }
            } else if ft.is_file() {
                LinkSlot::RegularFile
            } else if ft.is_dir() {
                if dir_is_empty(link) {
                    LinkSlot::EmptyRealDir
                } else {
                    LinkSlot::NonEmptyRealDir
                }
            } else {
                // sockets, fifos, etc. — treat as non-symlink we won't touch.
                LinkSlot::NonEmptyRealDir
            }
        }
    }
}

fn dir_is_empty(p: &Path) -> bool {
    fs::read_dir(p)
        .map(|mut it| it.next().is_none())
        .unwrap_or(false)
}

fn dir_contains_source(link: &Path, source_canon: &Path) -> bool {
    let link_canon = link.canonicalize().unwrap_or_else(|_| link.to_path_buf());
    is_inside(source_canon, &link_canon)
}

#[derive(Debug)]
enum LinkProbe {
    DoesNotExist,
    NotASymlink,
    Symlink {
        target_exists: bool,
        target_abs: PathBuf,
        target_canonical: Option<PathBuf>,
    },
}

fn read_link_target(link: &Path) -> LinkProbe {
    match fs::symlink_metadata(link) {
        Err(_) => LinkProbe::DoesNotExist,
        Ok(m) => {
            if !m.file_type().is_symlink() {
                return LinkProbe::NotASymlink;
            }
            match fs::read_link(link) {
                Ok(t) => {
                    let abs = if t.is_absolute() {
                        t
                    } else {
                        link.parent().map(|p| p.join(&t)).unwrap_or(t)
                    };
                    let canon = abs.canonicalize().ok();
                    let exists = canon.is_some();
                    LinkProbe::Symlink {
                        target_exists: exists,
                        target_abs: abs.clone(),
                        target_canonical: canon,
                    }
                }
                Err(_) => LinkProbe::Symlink {
                    target_exists: false,
                    target_abs: PathBuf::new(),
                    target_canonical: None,
                },
            }
        }
    }
}

// -----------------------------------------------------------------------------
// Topology safety per SPEC §"符号链接安全语义"
// -----------------------------------------------------------------------------

fn check_topology(link: &Path, source_canon: &Path) -> Result<(), XdError> {
    // 1. Same object: does the link, after symlink expansion of any
    //    existing parent component, resolve to the source itself?
    if let Ok(link_canon) = link.canonicalize() {
        if link_canon == source_canon {
            return Err(XdError::planning(format!(
                "链接路径与源路径解析为同一对象: {}",
                source_canon.display()
            )));
        }
        // Link is inside source.
        if is_inside(&link_canon, source_canon) && link_canon != *source_canon {
            return Err(XdError::planning(format!(
                "链接路径位于源路径内部: {} 在 {} 之内",
                link_canon.display(),
                source_canon.display()
            )));
        }
    } else {
        // Link doesn't exist yet: first check lexical path prefix
        // against source_canon. Since source paths are validated to
        // have no symlink components, lexical == canonical for the
        // source side. This catches cases where intermediate ancestors
        // also don't exist (e.g. link = source/dir/sub/file, but sub/
        // hasn't been created yet).
        if link.starts_with(source_canon) && link != source_canon {
            return Err(XdError::planning(format!(
                "链接路径位于源路径内部: {} 在 {} 之内",
                link.display(),
                source_canon.display()
            )));
        }
        // Then check the lexical relationship after resolving any
        // parent directory that does exist.
        if let Some(parent) = link.parent() {
            if let Ok(parent_canon) = parent.canonicalize() {
                let final_pos = link
                    .file_name()
                    .map(|n| parent_canon.join(n))
                    .unwrap_or(parent_canon.clone());
                if final_pos == *source_canon || is_inside(&final_pos, source_canon) {
                    return Err(XdError::planning(format!(
                        "链接路径会落在源路径内部或等于源路径: {}",
                        source_canon.display()
                    )));
                }
                // Source under link's existing parent path means an
                // empty-dir replacement could remove/contain source.
                // dir_contains_source handles that case at action build time.
            }
        }
    }

    // 2. Unsafe ancestors: per SPEC §"符号链接安全语义", *any* existing
    //    ancestor of the link path being a non-directory (regular file)
    //    or being a symlink whose target lands the actual creation
    //    position in source territory must be rejected as a planning-
    //    block error.
    let mut cur = link.parent();
    while let Some(c) = cur {
        let Ok(meta) = fs::symlink_metadata(c) else {
            // Ancestor doesn't exist on disk yet; further ancestors don't
            // matter (they'll be created or also missing).
            cur = c.parent();
            if cur == Some(c) {
                break;
            }
            continue;
        };
        let ft = meta.file_type();
        if ft.is_symlink() {
            // A symlinked ancestor is unsafe iff the actual creation
            // position (link path "rebased" via this symlink) lands at
            // or inside the source. Compute the rebased final position:
            // rest = link's path beneath this ancestor; final_pos =
            // ancestor_canonical / rest.
            if let Ok(rest) = link.strip_prefix(c) {
                if let Ok(target) = fs::read_link(c) {
                    let target_abs = if target.is_absolute() {
                        target
                    } else {
                        c.parent().map(|p| p.join(&target)).unwrap_or(target)
                    };
                    if let Ok(canon) = target_abs.canonicalize() {
                        let final_pos = canon.join(rest);
                        if final_pos == *source_canon || is_inside(&final_pos, source_canon) {
                            return Err(XdError::planning(format!(
                                "链接路径祖先 {} 是不安全的符号链接 (-> {})",
                                c.display(),
                                canon.display()
                            )));
                        }
                    }
                }
            }
        } else if !ft.is_dir() {
            // Non-directory ancestor (regular file, fifo, socket...).
            return Err(XdError::planning(format!(
                "链接路径祖先 {} 不是目录",
                c.display()
            )));
        }
        // Move to the next existing ancestor.
        let next = c.parent();
        if next == Some(c) {
            break;
        }
        cur = next;
    }

    // 3. Symlink loop: if a parent ancestor is a symlink whose target is
    //    in the same chain (link path → ... → link path), reject.
    if would_create_loop(link, source_canon) {
        return Err(XdError::planning(format!(
            "创建符号链接 {} -> {} 会产生符号链接循环",
            link.display(),
            source_canon.display()
        )));
    }

    Ok(())
}

fn would_create_loop(link: &Path, source_canon: &Path) -> bool {
    // Walk from link's parent upwards; if any ancestor is a symlink
    // whose canonical target equals or contains the link-creation site,
    // we'd loop.
    let mut cur = link.parent();
    while let Some(c) = cur {
        if let Ok(meta) = fs::symlink_metadata(c) {
            if meta.file_type().is_symlink() {
                if let Ok(t) = fs::read_link(c) {
                    let abs = if t.is_absolute() {
                        t
                    } else {
                        c.parent().map(|p| p.join(&t)).unwrap_or(t)
                    };
                    if let Ok(canon) = abs.canonicalize() {
                        if let Ok(link_canon) = link.canonicalize() {
                            if is_inside(&canon, &link_canon) {
                                return true;
                            }
                        }
                        // also consider source loop
                        if let Ok(rel) = source_canon.strip_prefix(&canon) {
                            let _ = rel;
                            // Source under symlinked ancestor isn't itself a loop;
                            // skip false positive.
                        }
                    }
                }
            }
        }
        if c.parent() == Some(c) {
            break;
        }
        cur = c.parent();
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(not(windows))]
    use crate::config::Config;
#[cfg(not(windows))]
    use std::sync::atomic::{AtomicU64, Ordering};

    #[cfg(not(windows))]
    static C: AtomicU64 = AtomicU64::new(0);

    #[cfg(not(windows))]
    fn tmpdir(tag: &str) -> PathBuf {
        let id = C.fetch_add(1, Ordering::SeqCst);
        let p = std::env::temp_dir().join(format!("xd_plan_{}_{}_{}", tag, std::process::id(), id));
        let _ = fs::remove_dir_all(&p);
        fs::create_dir_all(&p).unwrap();
        p
    }

    #[cfg(not(windows))]
    fn dc(dir: &Path, links: Vec<(&str, &str)>) -> DiscoveredConfig {
        let mut m = BTreeMap::new();
        for (k, v) in links {
            m.insert(k.to_string(), v.to_string());
        }
        DiscoveredConfig {
            config_file: dir.join("xdotter.toml"),
            config_dir: dir.canonicalize().unwrap_or_else(|_| dir.to_path_buf()),
            config: Config {
                links: m,
                dependencies: BTreeMap::new(),
            },
        }
    }

    #[test]
    #[cfg(not(windows))]
    fn collision_lists_all_conflicting_entries() {
        std::env::set_var("HOME", "/tmp");
        let d1 = tmpdir("col1");
        let d2 = tmpdir("col2");
        fs::write(d1.join("a.txt"), "a").unwrap();
        fs::write(d2.join("b.txt"), "b").unwrap();
        let confs = vec![
            dc(&d1, vec![("a.txt", "/tmp/xd_collide.txt")]),
            dc(&d2, vec![("b.txt", "/tmp/xd_collide.txt")]),
        ];
        let mut errs = ErrorBag::new();
        let _ = collect_global_links(&confs, &mut errs);
        let v = errs.into_vec();
        assert_eq!(v.len(), 1);
        let msg = v[0].body();
        assert!(
            msg.contains("a.txt") && msg.contains("b.txt"),
            "message must list all conflicting entries: {msg}"
        );
        assert!(v[0].is_config());
    }

    #[test]
    #[cfg(not(windows))]
    fn three_way_collision_lists_all_three() {
        std::env::set_var("HOME", "/tmp");
        let d1 = tmpdir("c3a");
        let d2 = tmpdir("c3b");
        let d3 = tmpdir("c3c");
        fs::write(d1.join("x"), "x").unwrap();
        fs::write(d2.join("y"), "y").unwrap();
        fs::write(d3.join("z"), "z").unwrap();
        let confs = vec![
            dc(&d1, vec![("x", "/tmp/xd_3way.txt")]),
            dc(&d2, vec![("y", "/tmp/xd_3way.txt")]),
            dc(&d3, vec![("z", "/tmp/xd_3way.txt")]),
        ];
        let mut errs = ErrorBag::new();
        let _ = collect_global_links(&confs, &mut errs);
        let v = errs.into_vec();
        assert_eq!(v.len(), 1);
        let msg = v[0].body();
        for nm in ["x", "y", "z"] {
            assert!(msg.contains(nm), "missing {nm} in: {msg}");
        }
    }

    #[test]
    #[cfg(not(windows))]
    fn missing_source_yields_planning_error() {
        std::env::set_var("HOME", "/tmp");
        let d = tmpdir("nosrc");
        let confs = vec![dc(&d, vec![("ghost.txt", "/tmp/xd_nosrc_target")])];
        let mut errs = ErrorBag::new();
        let entries = collect_global_links(&confs, &mut errs);
        assert!(errs.is_empty());
        let mut errs2 = ErrorBag::new();
        for e in entries {
            if let Err(err) = plan_one_deploy(&e, ConflictMode::Default) {
                errs2.push(err);
            }
        }
        assert!(errs2.iter().any(|e| e.is_planning()));
    }
}
