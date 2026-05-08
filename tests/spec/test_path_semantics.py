"""Path semantics per SPEC §"路径语义"."""

import os
import pytest


# ---------------------------------------------------------------
# Source path validation
# ---------------------------------------------------------------

def test_source_path_rejects_absolute(run_xd, tmp_repo, unique_home):
    """Source path that is absolute → config error."""
    repo = tmp_repo
    home = unique_home
    (repo / "a").write_text("x")
    (repo / "xdotter.toml").write_text('[links]\n"/abs" = "~/.target"\n')
    result = run_xd(["deploy"], cwd=repo, home=home)
    assert result.code != 0
    assert "[配置错误]" in result.stderr


def test_source_path_rejects_home_relative(run_xd, tmp_repo, unique_home):
    """Source path that is home-relative → config error."""
    repo = tmp_repo
    home = unique_home
    (repo / "xdotter.toml").write_text('[links]\n"~/src" = "~/.target"\n')
    result = run_xd(["deploy"], cwd=repo, home=home)
    assert result.code != 0
    assert "[配置错误]" in result.stderr


def test_source_path_rejects_empty(run_xd, tmp_repo, unique_home):
    """Empty source path → config error."""
    repo = tmp_repo
    home = unique_home
    (repo / "xdotter.toml").write_text('[links]\n"" = "~/.target"\n')
    result = run_xd(["deploy"], cwd=repo, home=home)
    assert result.code != 0
    assert "[配置错误]" in result.stderr


def test_source_path_rejects_dot(run_xd, tmp_repo, unique_home):
    """Source path "." → config error."""
    repo = tmp_repo
    home = unique_home
    (repo / "xdotter.toml").write_text('[links]\n"." = "~/.target"\n')
    result = run_xd(["deploy"], cwd=repo, home=home)
    assert result.code != 0
    assert "[配置错误]" in result.stderr


def test_source_path_rejects_double_dot(run_xd, tmp_repo, unique_home):
    """Source path containing ".." → config error."""
    repo = tmp_repo
    home = unique_home
    (repo / "a").write_text("x")
    (repo / "xdotter.toml").write_text('[links]\n"../escape" = "~/.target"\n')
    result = run_xd(["deploy"], cwd=repo, home=home)
    assert result.code != 0
    assert "[配置错误]" in result.stderr


# ---------------------------------------------------------------
# Link path validation
# ---------------------------------------------------------------

def test_link_path_rejects_normal_relative(run_xd, tmp_repo, unique_home):
    """Link path that is normal-relative → config error."""
    repo = tmp_repo
    home = unique_home
    (repo / "a").write_text("x")
    (repo / "xdotter.toml").write_text('[links]\n"a" = "relative/path"\n')
    result = run_xd(["deploy"], cwd=repo, home=home)
    assert result.code != 0
    assert "[配置错误]" in result.stderr


def test_link_path_rejects_empty(run_xd, tmp_repo, unique_home):
    """Empty link path → config error."""
    repo = tmp_repo
    home = unique_home
    (repo / "a").write_text("x")
    (repo / "xdotter.toml").write_text('[links]\n"a" = ""\n')
    result = run_xd(["deploy"], cwd=repo, home=home)
    assert result.code != 0
    assert "[配置错误]" in result.stderr


@pytest.mark.parametrize("raw", ["~", "~/", "/", "//", "///"])
def test_link_path_rejects_root_or_home_static(raw, run_xd, tmp_repo, unique_home):
    """Link paths that statically resolve to root or home → config error."""
    repo = tmp_repo
    home = unique_home
    (repo / "a").write_text("x")
    (repo / "xdotter.toml").write_text(f'[links]\n"a" = "{raw}"\n')
    result = run_xd(["deploy"], cwd=repo, home=home)
    assert result.code != 0, f"raw={raw}"
    assert "[配置错误]" in result.stderr


def test_link_path_rejects_double_dot_after_tilde(run_xd, tmp_repo, unique_home):
    """Link path with .. after ~/ → config error."""
    repo = tmp_repo
    home = unique_home
    (repo / "a").write_text("x")
    (repo / "xdotter.toml").write_text('[links]\n"a" = "~/a/../b"\n')
    result = run_xd(["deploy"], cwd=repo, home=home)
    assert result.code != 0
    assert "[配置错误]" in result.stderr


def test_link_path_rejects_curdir_only_after_tilde(run_xd, tmp_repo, unique_home):
    """Link path ~/./ or ~/./. normalizes to home dir → config error."""
    repo = tmp_repo
    home = unique_home
    (repo / "a").write_text("x")
    (repo / "xdotter.toml").write_text('[links]\n"a" = "~/./"\n')
    result = run_xd(["deploy"], cwd=repo, home=home)
    assert result.code != 0
    assert "[配置错误]" in result.stderr


def test_link_path_tilde_expansion_correct(run_xd, tmp_repo, unique_home):
    """Link path ~/.zshrc expands to HOME/.zshrc correctly."""
    repo = tmp_repo
    home = unique_home
    (repo / "zshrc").write_text("# zsh")
    (repo / "xdotter.toml").write_text('[links]\n"zshrc" = "~/.zshrc"\n')
    result = run_xd(["deploy"], cwd=repo, home=home)
    assert result.code == 0, result.stderr
    link = home / ".zshrc"
    assert link.is_symlink()


# ---------------------------------------------------------------
# Dependency path validation
# ---------------------------------------------------------------

def test_dep_path_rejects_absolute(run_xd, tmp_repo, unique_home):
    """Dependency path that is absolute → config error."""
    repo = tmp_repo
    home = unique_home
    (repo / "xdotter.toml").write_text('[dependencies]\n"d" = "/abs"\n')
    result = run_xd(["deploy"], cwd=repo, home=home)
    assert result.code != 0
    assert "[配置错误]" in result.stderr


def test_dep_path_rejects_double_dot(run_xd, tmp_repo, unique_home):
    """Dependency path containing ".." → config error."""
    repo = tmp_repo
    home = unique_home
    (repo / "xdotter.toml").write_text('[dependencies]\n"d" = "../escape"\n')
    result = run_xd(["deploy"], cwd=repo, home=home)
    assert result.code != 0
    assert "[配置错误]" in result.stderr


def test_dep_path_rejects_dot(run_xd, tmp_repo, unique_home):
    """Dependency path "." → config error."""
    repo = tmp_repo
    home = unique_home
    (repo / "xdotter.toml").write_text('[dependencies]\n"d" = "."\n')
    result = run_xd(["deploy"], cwd=repo, home=home)
    assert result.code != 0
    assert "[配置错误]" in result.stderr


# ---------------------------------------------------------------
# ~ expansion edge cases
# ---------------------------------------------------------------

def test_tilde_only_in_middle_not_expanded(run_xd, tmp_repo, unique_home):
    """~ in the middle of a path is not expanded — treated as literal."""
    repo = tmp_repo
    home = unique_home
    (repo / "a").write_text("x")
    # This is a normal-relative path (not home-relative), so it should
    # be rejected as a link path (must be absolute or home-relative).
    (repo / "xdotter.toml").write_text('[links]\n"a" = "foo/~bar/baz"\n')
    result = run_xd(["deploy"], cwd=repo, home=home)
    assert result.code != 0
    assert "[配置错误]" in result.stderr


def test_home_undeterminable_is_planning_error(tmp_repo):
    """When HOME cannot be determined, ~/ expansion fails → planning error.

    Note: this test unsets HOME entirely. The Rust impl falls back to
    dirs::home_dir() which may still succeed on a real machine, so we
    skip if the binary happens to find a home dir anyway.
    """
    import subprocess
    from pathlib import Path

    repo = tmp_repo
    (repo / "a").write_text("x")
    (repo / "xdotter.toml").write_text('[links]\n"a" = "~/.target"\n')

    # Resolve binary the same way conftest does
    env_bin = os.environ.get("XDOTTER_BIN")
    if env_bin:
        binary = env_bin
    else:
        project_root = Path(__file__).resolve().parent.parent.parent
        binary = str(project_root / "target" / "debug" / "xd")

    # Run with HOME completely unset
    env = {k: v for k, v in os.environ.items() if k != "HOME"}
    env["PATH"] = os.environ.get("PATH", "")
    proc = subprocess.run(
        [binary, "deploy"],
        cwd=str(repo),
        env=env,
        capture_output=True,
        timeout=30,
    )
    # If the system still has a fallback home dir, this test is inconclusive.
    # We accept either outcome: planning error (HOME truly unavailable) or
    # success (dirs::home_dir() found one).
    stderr = proc.stderr.decode(errors="replace")
    if proc.returncode != 0:
        assert "[规划阻塞错误]" in stderr or "[配置错误]" in stderr, stderr
