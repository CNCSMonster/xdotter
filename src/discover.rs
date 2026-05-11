//! Recursive configuration discovery per SPEC §"依赖语义".
//!
//! Walks from a root `xdotter.toml`, following `[dependencies]` to build
//! the global set of reachable configurations. Each configuration owns
//! its own *configuration directory tree* (its directory + descendants);
//! source paths and dependency paths declared in that toml must remain
//! inside that tree after resolution. Shared dependencies (same canonical
//! directory) are processed once.
//!
//! This stage emits configuration errors and planning-block errors only;
//! it does not modify the filesystem.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use crate::config::Config;
use crate::error::{decorate, ErrorBag, XdError};
use crate::path as p;

/// A configuration that has been discovered, parsed, and had its
/// dependency paths statically validated. Source and link path strings
/// are kept *raw* — full validation happens in the planning stage where
/// filesystem state is consulted.
#[derive(Debug, Clone)]
pub struct DiscoveredConfig {
    /// Absolute path to this config's `xdotter.toml`.
    pub config_file: PathBuf,
    /// Absolute, canonical path of the directory containing the toml.
    /// All source/dependency paths in this config are resolved against
    /// this directory and must remain inside its tree.
    pub config_dir: PathBuf,
    /// Parsed config (links + dependencies, both possibly empty).
    pub config: Config,
}

/// Discovery result. Configs are returned in a stable order: root first,
/// then dependencies in deterministic (sorted) traversal order.
#[derive(Debug, Default)]
pub struct Discovered {
    pub configs: Vec<DiscoveredConfig>,
    pub errors: ErrorBag,
}

/// Entry point. `root` must be a directory containing `xdotter.toml`.
pub fn discover(root: &Path) -> Discovered {
    let mut out = Discovered::default();
    let mut seen: BTreeSet<PathBuf> = BTreeSet::new();
    let mut stack: BTreeSet<PathBuf> = BTreeSet::new();

    let root_canon = match canonicalize_dir(root) {
        Ok(p) => p,
        Err(e) => {
            out.errors.push(XdError::planning(format!(
                "无法访问根配置目录 {}: {}",
                root.display(),
                e
            )));
            return out;
        }
    };
    visit(&root_canon, &mut seen, &mut stack, &mut out);
    out
}

fn visit(
    dir: &Path,
    seen: &mut BTreeSet<PathBuf>,
    stack: &mut BTreeSet<PathBuf>,
    out: &mut Discovered,
) {
    // Check the active DFS stack BEFORE the seen set: a directory may
    // legitimately be a shared dependency (visited once, processed,
    // popped from stack) — in which case we skip silently. But a cycle
    // is "directory still on the active path", which must be reported
    // even if we've also added it to `seen`.
    if stack.contains(dir) {
        out.errors.push(XdError::planning(format!(
            "依赖图存在真实循环: {} 在遍历中再次出现",
            dir.display()
        )));
        return;
    }
    if seen.contains(dir) {
        // Shared dependency, already processed.
        return;
    }
    stack.insert(dir.to_path_buf());

    let toml_path = dir.join("xdotter.toml");
    let content = match fs::read_to_string(&toml_path) {
        Ok(s) => s,
        Err(e) => {
            out.errors.push(XdError::planning(format!(
                "无法读取配置文件 {}: {}",
                toml_path.display(),
                e
            )));
            stack.remove(dir);
            return;
        }
    };

    let cfg = match Config::from_toml(&content, &toml_path) {
        Ok(c) => c,
        Err(e) => {
            out.errors.push(e);
            seen.insert(dir.to_path_buf());
            stack.remove(dir);
            return;
        }
    };

    // Static dependency-path validation, plus same-table real-dir uniqueness.
    let mut resolved_in_table: BTreeMap<PathBuf, String> = BTreeMap::new();
    let mut to_recurse: Vec<PathBuf> = Vec::new();

    for (name, raw) in &cfg.dependencies {
        if let Err(e) = p::validate_dependency_path(raw) {
            out.errors
                .push(decorate(&e, &toml_path, Some(&format!("依赖 \"{name}\""))));
            continue;
        }
        let dep_dir = dir.join(raw);
        let dep_canon = match canonicalize_dir(&dep_dir) {
            Ok(c) => c,
            Err(e) => {
                out.errors.push(XdError::planning(format!(
                    "{}: 依赖 \"{}\" 路径不存在或无法访问 ({}): {}",
                    toml_path.display(),
                    name,
                    raw,
                    e
                )));
                continue;
            }
        };
        if !is_inside(&dep_canon, dir) {
            out.errors.push(XdError::planning(format!(
                "{}: 依赖 \"{}\" 解析后逃出当前配置目录树: {}",
                toml_path.display(),
                name,
                raw
            )));
            continue;
        }
        // Per-table uniqueness.
        if let Some(prev) = resolved_in_table.get(&dep_canon) {
            out.errors.push(XdError::config(format!(
                "{}: 同一 [dependencies] 表中多个依赖解析到同一真实目录: \"{}\" 与 \"{}\" 都指向 {}",
                toml_path.display(),
                prev,
                name,
                dep_canon.display()
            )));
            continue;
        }
        resolved_in_table.insert(dep_canon.clone(), name.clone());
        // Must contain its own xdotter.toml.
        if !dep_canon.join("xdotter.toml").exists() {
            out.errors.push(XdError::planning(format!(
                "{}: 依赖 \"{}\" 目录缺少 xdotter.toml: {}",
                toml_path.display(),
                name,
                dep_canon.display()
            )));
            continue;
        }
        to_recurse.push(dep_canon);
    }

    out.configs.push(DiscoveredConfig {
        config_file: toml_path,
        config_dir: dir.to_path_buf(),
        config: cfg,
    });
    seen.insert(dir.to_path_buf());

    for d in to_recurse {
        visit(&d, seen, stack, out);
    }

    stack.remove(dir);
}

fn canonicalize_dir(p: &Path) -> std::io::Result<PathBuf> {
    p.canonicalize().and_then(|c| {
        if c.is_dir() {
            Ok(c)
        } else {
            Err(std::io::Error::new(
                std::io::ErrorKind::NotADirectory,
                format!("{} 不是目录", c.display()),
            ))
        }
    })
}

/// True iff `child` is `parent` or a descendant of `parent`. Both must
/// be already-canonicalized absolute paths.
pub fn is_inside(child: &Path, parent: &Path) -> bool {
    child.starts_with(parent)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static C: AtomicU64 = AtomicU64::new(0);

    fn tmpdir(tag: &str) -> PathBuf {
        let id = C.fetch_add(1, Ordering::SeqCst);
        let p = std::env::temp_dir().join(format!("xd_disc_{}_{}_{}", tag, std::process::id(), id));
        let _ = fs::remove_dir_all(&p);
        fs::create_dir_all(&p).unwrap();
        p
    }

    #[test]
    fn empty_root_config_succeeds() {
        let d = tmpdir("empty");
        fs::write(d.join("xdotter.toml"), "").unwrap();
        let r = discover(&d);
        assert!(
            r.errors.is_empty(),
            "errors: {:?}",
            r.errors.iter().collect::<Vec<_>>()
        );
        assert_eq!(r.configs.len(), 1);
    }

    #[test]
    fn linear_dependency() {
        let d = tmpdir("linear");
        fs::create_dir_all(d.join("sub")).unwrap();
        fs::write(d.join("sub/xdotter.toml"), "").unwrap();
        fs::write(
            d.join("xdotter.toml"),
            r#"
[dependencies]
"sub" = "sub"
"#,
        )
        .unwrap();
        let r = discover(&d);
        assert!(r.errors.is_empty());
        assert_eq!(r.configs.len(), 2);
    }

    #[test]
    fn dep_outside_tree_is_planning_error() {
        let outer = tmpdir("outer_root");
        let sibling = tmpdir("sibling");
        fs::write(sibling.join("xdotter.toml"), "").unwrap();
        // `outer` references `sibling` via `..`
        // Writing the relative path is invalid statically (..); use a
        // symlink to exercise tree-escape detection instead.
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(&sibling, outer.join("escape")).unwrap();
        }
        fs::write(
            outer.join("xdotter.toml"),
            r#"
[dependencies]
"e" = "escape"
"#,
        )
        .unwrap();

        let r = discover(&outer);
        assert!(!r.errors.is_empty(), "expected planning error");
        assert!(r.errors.iter().any(|e| e.is_planning()));
    }

    #[test]
    fn dotdot_dep_path_is_config_error() {
        let d = tmpdir("dotdot");
        fs::write(
            d.join("xdotter.toml"),
            r#"
[dependencies]
"bad" = "../escape"
"#,
        )
        .unwrap();
        let r = discover(&d);
        assert!(r.errors.iter().any(|e| e.is_config()));
    }

    #[test]
    fn duplicate_real_dir_in_same_table_is_config_error() {
        let d = tmpdir("dup");
        fs::create_dir_all(d.join("real")).unwrap();
        fs::write(d.join("real/xdotter.toml"), "").unwrap();
        #[cfg(unix)]
        std::os::unix::fs::symlink(d.join("real"), d.join("alias")).unwrap();
        // Without symlink we couldn't trigger this on non-Unix easily;
        // skip the assertion there.
        #[cfg(unix)]
        {
            fs::write(
                d.join("xdotter.toml"),
                r#"
[dependencies]
"a" = "real"
"b" = "alias"
"#,
            )
            .unwrap();
            let r = discover(&d);
            assert!(
                r.errors.iter().any(|e| e.is_config()),
                "errors: {:?}",
                r.errors.iter().collect::<Vec<_>>()
            );
        }
    }

    #[test]
    fn shared_dep_processed_once() {
        let d = tmpdir("shared");
        fs::create_dir_all(d.join("a")).unwrap();
        fs::create_dir_all(d.join("b")).unwrap();
        fs::create_dir_all(d.join("shared")).unwrap();
        fs::write(d.join("shared/xdotter.toml"), "").unwrap();
        fs::write(
            d.join("a/xdotter.toml"),
            r#"
[dependencies]
"s" = "../shared"
"#,
        )
        .unwrap();
        // SPEC: dep paths must not contain `..`. So we exercise sharing via the root.
        // Root depends on both `a` and `shared`; `a` cannot legally point to `shared`
        // since that would require `..`. So just verify a/shared each appear once.
        fs::write(
            d.join("xdotter.toml"),
            r#"
[dependencies]
"shared" = "shared"
"a" = "a"
"#,
        )
        .unwrap();
        // Reset a/xdotter.toml to empty so it doesn't fail with `..` config error.
        fs::write(d.join("a/xdotter.toml"), "").unwrap();
        let r = discover(&d);
        assert!(
            r.errors.is_empty(),
            "errors: {:?}",
            r.errors.iter().collect::<Vec<_>>()
        );
        // Root + a + shared = 3 configs, no double-visit.
        assert_eq!(r.configs.len(), 3);
    }

    #[test]
    #[cfg(unix)]
    fn real_cycle_detected() {
        let d = tmpdir("cycle");
        let a = d.join("a");
        let sub = a.join("sub");
        fs::create_dir_all(&sub).unwrap();
        // a/sub/self_link -> a/sub  (cycle)
        std::os::unix::fs::symlink(&sub, sub.join("self_link")).unwrap();
        fs::write(
            a.join("xdotter.toml"),
            r#"
[dependencies]
"sub" = "sub"
"#,
        )
        .unwrap();
        fs::write(
            sub.join("xdotter.toml"),
            r#"
[dependencies]
"self" = "self_link"
"#,
        )
        .unwrap();

        let r = discover(&a);
        assert!(
            r.errors.iter().any(|e| e.is_planning()),
            "errors: {:?}",
            r.errors.iter().collect::<Vec<_>>()
        );
    }
}
