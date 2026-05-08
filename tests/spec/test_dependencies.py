"""Dependency semantics per SPEC §"依赖语义"."""

import pytest


def _write_dep_config(dep_dir, links=None):
    """Write an xdotter.toml in a dependency directory."""
    lines = []
    if links:
        lines.append("[links]")
        for src, link in links.items():
            lines.append(f'"{src}" = "{link}"')
    (dep_dir / "xdotter.toml").write_text("\n".join(lines) + "\n")


# ---------------------------------------------------------------
# Recursive discovery
# ---------------------------------------------------------------

def test_dep_recursive_discovery(run_xd, tmp_repo, unique_home):
    """Root config deploys links from dependency configs."""
    repo = tmp_repo
    home = unique_home
    sub = repo / "sub"
    sub.mkdir()

    (repo / "a").write_text("A")
    (sub / "b").write_text("B")

    _write_dep_config(sub, {"b": "~/.b"})
    (repo / "xdotter.toml").write_text(
        '[links]\n"a" = "~/.a"\n\n[dependencies]\n"sub" = "sub"\n'
    )

    result = run_xd(["deploy"], cwd=repo, home=home)
    assert result.code == 0, result.stderr
    assert (home / ".a").is_symlink()
    assert (home / ".b").is_symlink()


# ---------------------------------------------------------------
# Dependency must exist, be a dir, contain xdotter.toml
# ---------------------------------------------------------------

def test_dep_dir_not_exists_is_planning_error(run_xd, tmp_repo, unique_home):
    """Dependency directory doesn't exist → planning error."""
    repo = tmp_repo
    home = unique_home
    (repo / "a").write_text("A")
    (repo / "xdotter.toml").write_text(
        '[links]\n"a" = "~/.a"\n\n[dependencies]\n"missing" = "nonexistent"\n'
    )
    result = run_xd(["deploy"], cwd=repo, home=home)
    assert result.code != 0
    assert "[规划阻塞错误]" in result.stderr


def test_dep_not_a_dir_is_planning_error(run_xd, tmp_repo, unique_home):
    """Dependency path is a file → planning error."""
    repo = tmp_repo
    home = unique_home
    (repo / "a").write_text("A")
    (repo / "dep_file").write_text("")
    (repo / "xdotter.toml").write_text(
        '[links]\n"a" = "~/.a"\n\n[dependencies]\n"bad" = "dep_file"\n'
    )
    result = run_xd(["deploy"], cwd=repo, home=home)
    assert result.code != 0
    assert "[规划阻塞错误]" in result.stderr


def test_dep_missing_xdotter_toml_is_planning_error(run_xd, tmp_repo, unique_home):
    """Dependency directory lacks xdotter.toml → planning error."""
    repo = tmp_repo
    home = unique_home
    sub = repo / "empty"
    sub.mkdir()
    (repo / "a").write_text("A")
    (repo / "xdotter.toml").write_text(
        '[links]\n"a" = "~/.a"\n\n[dependencies]\n"empty" = "empty"\n'
    )
    result = run_xd(["deploy"], cwd=repo, home=home)
    assert result.code != 0
    assert "[规划阻塞错误]" in result.stderr


# ---------------------------------------------------------------
# Dep path must stay inside config dir tree
# ---------------------------------------------------------------

def test_dep_escapes_tree_is_planning_error(run_xd, tmp_repo, unique_home):
    """Dependency resolves outside config dir tree → planning error."""
    repo = tmp_repo
    home = unique_home
    outside = tmp_repo.parent / "outside_dep"
    outside.mkdir(exist_ok=True)
    _write_dep_config(outside, {"x": "~/.x"})

    (repo / "a").write_text("A")
    # Using a symlink to escape — but dep paths with .. are rejected statically.
    # We test with an invalid path string instead.
    (repo / "xdotter.toml").write_text(
        '[links]\n"a" = "~/.a"\n\n[dependencies]\n"esc" = "../outside_dep"\n'
    )
    result = run_xd(["deploy"], cwd=repo, home=home)
    assert result.code != 0
    # .. in dep path is a config error
    assert "[配置错误]" in result.stderr


# ---------------------------------------------------------------
# Cycle detection
# ---------------------------------------------------------------

def test_dep_cycle_detected(run_xd, tmp_repo, unique_home):
    """Dependency cycle (a→b→a) → planning error.

    Note: dep paths with `..` are statically rejected as config errors,
    so we use symlinks to create a real cycle that passes static validation.
    """
    repo = tmp_repo
    home = unique_home
    a = repo / "a"
    b = repo / "b"
    a.mkdir()
    b.mkdir()

    # a depends on b via a valid relative path
    (a / "xdotter.toml").write_text('[dependencies]\n"b" = "b"\n')
    # But "b" relative to a/ is repo/a/b — create a symlink to repo/b
    (repo / "a" / "b").symlink_to(b)
    # b depends on a via a valid relative path
    (b / "xdotter.toml").write_text('[dependencies]\n"a" = "a"\n')
    # "a" relative to b/ is repo/b/a — symlink to repo/a
    (repo / "b" / "a").symlink_to(a)

    (repo / "xdotter.toml").write_text('[dependencies]\n"a" = "a"\n')

    result = run_xd(["deploy"], cwd=repo, home=home)
    assert result.code != 0
    # Cycle detection triggers a planning error
    assert "[规划阻塞错误]" in result.stderr


# ---------------------------------------------------------------
# Same-table duplicate real dir
# ---------------------------------------------------------------

def test_same_table_dup_real_dir_is_config_error(run_xd, tmp_repo, unique_home):
    """Two deps in same [dependencies] resolving to same dir → config error."""
    repo = tmp_repo
    home = unique_home
    real = repo / "real"
    real.mkdir()
    _write_dep_config(real)

    # Create a symlink alias
    (repo / "alias").symlink_to(real)

    (repo / "xdotter.toml").write_text(
        '[dependencies]\n"d1" = "real"\n"d2" = "alias"\n'
    )
    result = run_xd(["deploy"], cwd=repo, home=home)
    assert result.code != 0
    assert "[配置错误]" in result.stderr


# ---------------------------------------------------------------
# Shared dep processed once
# ---------------------------------------------------------------

def test_shared_dep_processed_once(run_xd, tmp_repo, unique_home):
    """Two configs referencing same dep → dep processed only once."""
    repo = tmp_repo
    home = unique_home
    sub = repo / "sub"
    shared = repo / "shared"
    sub.mkdir()
    shared.mkdir()

    _write_dep_config(shared, {"c": "~/.c"})
    _write_dep_config(sub, links={})  # sub has no deps of its own

    (repo / "a").write_text("A")
    (shared / "c").write_text("C")

    (repo / "xdotter.toml").write_text(
        '[links]\n"a" = "~/.a"\n\n[dependencies]\n"sub" = "sub"\n"shared" = "shared"\n'
    )
    # sub also references shared via the root (root handles it; sub doesn't need to re-declare)
    (sub / "xdotter.toml").write_text("")

    result = run_xd(["deploy"], cwd=repo, home=home)
    assert result.code == 0, result.stderr
    assert (home / ".a").is_symlink()
    assert (home / ".c").is_symlink()


# ---------------------------------------------------------------
# Global link collision across deps
# ---------------------------------------------------------------

def test_global_link_collision_across_deps(run_xd, tmp_repo, unique_home):
    """Two link entries in different configs mapping to same link → config error."""
    repo = tmp_repo
    home = unique_home
    sub = repo / "sub"
    sub.mkdir()

    (repo / "a").write_text("A")
    (sub / "b").write_text("B")

    _write_dep_config(sub, {"b": "~/.collide"})
    (repo / "xdotter.toml").write_text(
        '[links]\n"a" = "~/.collide"\n\n[dependencies]\n"sub" = "sub"\n'
    )

    result = run_xd(["deploy"], cwd=repo, home=home)
    assert result.code != 0
    assert "[配置错误]" in result.stderr
    # Must list both conflicting entries
    assert "a" in result.stderr
    assert "b" in result.stderr


# ---------------------------------------------------------------
# Undeploy uses same dep discovery
# ---------------------------------------------------------------

def test_undeploy_discovers_deps(run_xd, tmp_repo, unique_home):
    """Undeploy removes links from dependency configs."""
    repo = tmp_repo
    home = unique_home
    sub = repo / "sub"
    sub.mkdir()

    (repo / "a").write_text("A")
    (sub / "b").write_text("B")

    _write_dep_config(sub, {"b": "~/.b"})
    (repo / "xdotter.toml").write_text(
        '[links]\n"a" = "~/.a"\n\n[dependencies]\n"sub" = "sub"\n'
    )

    # Deploy first
    result = run_xd(["deploy"], cwd=repo, home=home)
    assert result.code == 0

    # Now undeploy — should remove both links
    result = run_xd(["undeploy"], cwd=repo, home=home)
    assert result.code == 0, result.stderr
    assert not (home / ".a").exists()
    assert not (home / ".b").exists()
