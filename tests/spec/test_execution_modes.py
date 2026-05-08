"""Execution model per SPEC §"执行模型" — conflict modes × execution modes."""

import pytest


# ---------------------------------------------------------------
# Deploy behavior table — link path states × modes
# ---------------------------------------------------------------

def _setup_deploy_repo(repo, home):
    """Create a repo with two links for testing."""
    (repo / "a").write_text("A")
    (repo / "b").write_text("B")
    (repo / "xdotter.toml").write_text(
        '[links]\n"a" = "~/.a"\n"b" = "~/.b"\n'
    )


def test_link_not_exists_all_modes_create(run_xd, tmp_repo, unique_home):
    """Link path doesn't exist → all modes create symlink."""
    repo = tmp_repo
    home = unique_home
    _setup_deploy_repo(repo, home)

    for extra_args in [[], ["--force"], ["--interactive"]]:
        # Clean state
        for p in [home / ".a", home / ".b"]:
            if p.exists() or p.is_symlink():
                p.unlink()
        result = run_xd(["deploy"] + extra_args, cwd=repo, home=home)
        assert result.code == 0, f"{extra_args}: {result.stderr}"
        assert (home / ".a").is_symlink()
        assert (home / ".b").is_symlink()


def test_correct_symlink_all_modes_skip(run_xd, tmp_repo, unique_home):
    """Already correct symlink → all modes skip,视为 success."""
    repo = tmp_repo
    home = unique_home
    _setup_deploy_repo(repo, home)

    # Deploy once
    result = run_xd(["deploy"], cwd=repo, home=home)
    assert result.code == 0

    # Now deploy again in all modes — should skip
    for extra_args in [[], ["--force"], ["--interactive"]]:
        result = run_xd(["deploy"] + extra_args, cwd=repo, home=home)
        assert result.code == 0, f"{extra_args}: {result.stderr}"
        assert (home / ".a").is_symlink()


def test_regular_file_force_replaces(run_xd, tmp_repo, unique_home):
    """Regular file at link path → force replaces."""
    repo = tmp_repo
    home = unique_home
    _setup_deploy_repo(repo, home)

    # Pre-populate as regular file
    (home / ".a").write_text("stale")
    result = run_xd(["deploy", "--force"], cwd=repo, home=home)
    assert result.code == 0, result.stderr
    assert (home / ".a").is_symlink()


def test_regular_file_default_skips(run_xd, tmp_repo, unique_home):
    """Regular file at link path → default mode skips, counts as failure."""
    repo = tmp_repo
    home = unique_home
    _setup_deploy_repo(repo, home)

    (home / ".a").write_text("stale")
    result = run_xd(["deploy"], cwd=repo, home=home)
    assert result.code != 0  # at least one link failed
    # .a should NOT be a symlink (skipped)
    assert not (home / ".a").is_symlink()
    # .b should still be deployed (continues after skipped link)
    assert (home / ".b").is_symlink()


def test_wrong_symlink_force_replaces(run_xd, tmp_repo, unique_home):
    """Wrong symlink → force replaces."""
    repo = tmp_repo
    home = unique_home
    _setup_deploy_repo(repo, home)

    # Create wrong symlink
    (home / ".a").symlink_to("/tmp")
    result = run_xd(["deploy", "--force"], cwd=repo, home=home)
    assert result.code == 0, result.stderr
    assert (home / ".a").is_symlink()
    # Target should point to repo/a, not /tmp
    link_target = (home / ".a").resolve()
    assert "a" in str(link_target)


def test_broken_symlink_force_replaces(run_xd, tmp_repo, unique_home):
    """Broken symlink → force replaces."""
    repo = tmp_repo
    home = unique_home
    _setup_deploy_repo(repo, home)

    # Create broken symlink (target doesn't exist)
    (home / ".a").symlink_to("/nonexistent_target_xyz")
    result = run_xd(["deploy", "--force"], cwd=repo, home=home)
    assert result.code == 0, result.stderr
    assert (home / ".a").is_symlink()
    link_target = (home / ".a").resolve()
    assert "a" in str(link_target)


def test_empty_dir_force_replaces(run_xd, tmp_repo, unique_home):
    """Empty real directory → force replaces (if safe)."""
    repo = tmp_repo
    home = unique_home
    _setup_deploy_repo(repo, home)

    (home / ".a").mkdir()
    result = run_xd(["deploy", "--force"], cwd=repo, home=home)
    assert result.code == 0, result.stderr
    assert (home / ".a").is_symlink()


def test_nonempty_dir_all_modes_skip(run_xd, tmp_repo, unique_home):
    """Non-empty real directory → all modes skip, counts as failure."""
    repo = tmp_repo
    home = unique_home
    _setup_deploy_repo(repo, home)

    (home / ".a").mkdir()
    (home / ".a" / "file").write_text("content")

    for extra_args in [[], ["--force"], ["--interactive"]]:
        result = run_xd(["deploy"] + extra_args, cwd=repo, home=home)
        assert result.code != 0
        # Should still be a directory, not a symlink
        assert (home / ".a").is_dir()
        assert not (home / ".a").is_symlink()
        # Clean up for next iteration
        (home / ".a" / "file").unlink()
        (home / ".a").rmdir()
        (home / ".a").mkdir()
        (home / ".a" / "file").write_text("content")


# ---------------------------------------------------------------
# Dry-run semantics
# ---------------------------------------------------------------

def test_dry_run_no_filesystem_modify(run_xd, tmp_repo, unique_home):
    """--dry-run does not modify filesystem."""
    repo = tmp_repo
    home = unique_home
    _setup_deploy_repo(repo, home)

    result = run_xd(["deploy", "--dry-run"], cwd=repo, home=home)
    assert result.code == 0
    assert not (home / ".a").exists()
    assert not (home / ".a").is_symlink()


def test_dry_run_same_validation_as_real(run_xd, tmp_repo, unique_home):
    """--dry-run performs same validation as real command."""
    repo = tmp_repo
    home = unique_home
    # Missing source → planning error in both modes
    (repo / "xdotter.toml").write_text('[links]\n"ghost" = "~/.ghost"\n')

    real_result = run_xd(["deploy"], cwd=repo, home=home)
    dry_result = run_xd(["deploy", "--dry-run"], cwd=repo, home=home)

    assert real_result.code != 0
    assert dry_result.code != 0
    # Both should be planning errors
    assert "[规划阻塞错误]" in real_result.stderr
    assert "[规划阻塞错误]" in dry_result.stderr


def test_force_dry_run_renders_replace(run_xd, tmp_repo, unique_home):
    """--force --dry-run shows 'replace' for recoverable conflicts."""
    repo = tmp_repo
    home = unique_home
    (repo / "a").write_text("A")
    (repo / "xdotter.toml").write_text('[links]\n"a" = "~/.a"\n')
    (home / ".a").write_text("stale")

    result = run_xd(["deploy", "--force", "--dry-run"], cwd=repo, home=home)
    assert result.code == 0
    assert "replace" in result.stdout.lower() or "替换" in result.stdout
    # FS untouched
    assert not (home / ".a").is_symlink()


def test_interactive_dry_run_renders_skip(run_xd, tmp_repo, unique_home):
    """--interactive --dry-run shows 'skip' / 'declined' for conflicts."""
    repo = tmp_repo
    home = unique_home
    (repo / "a").write_text("A")
    (repo / "xdotter.toml").write_text('[links]\n"a" = "~/.a"\n')
    (home / ".a").write_text("stale")

    result = run_xd(["deploy", "--interactive", "--dry-run"], cwd=repo, home=home)
    assert result.code != 0  # has skips counted as failures
    assert "declined" in result.stdout.lower() or "skip" in result.stdout.lower() or "跳过" in result.stdout
    # Must not promise a replace
    assert "replace" not in result.stdout.lower()
    # FS untouched
    assert not (home / ".a").is_symlink()


# ---------------------------------------------------------------
# Interactive mode — stdin handling
# ---------------------------------------------------------------

def test_interactive_non_tty_is_reject(run_xd, tmp_repo, unique_home):
    """--interactive with non-TTY stdin treats as reject."""
    repo = tmp_repo
    home = unique_home
    (repo / "a").write_text("A")
    (repo / "xdotter.toml").write_text('[links]\n"a" = "~/.a"\n')
    (home / ".a").write_text("stale")

    # subprocess always provides non-TTY stdin
    result = run_xd(["deploy", "--interactive"], cwd=repo, home=home)
    # Non-TTY = reject = link skipped = failure
    assert result.code != 0
    # FS untouched
    assert not (home / ".a").is_symlink()


def test_interactive_yes_confirms(run_xd, tmp_repo, unique_home):
    """--interactive with 'yes' on stdin — but non-TTY stdin is always rejected.

    Per SPEC: if stdin is not a TTY, treat as rejection. Since pytest
    always runs with non-TTY stdin, we verify the reject behavior.
    A real TTY test would require a pty harness (out of scope for now).
    """
    repo = tmp_repo
    home = unique_home
    (repo / "a").write_text("A")
    (repo / "xdotter.toml").write_text('[links]\n"a" = "~/.a"\n')
    (home / ".a").write_text("stale")

    # subprocess always provides non-TTY stdin → treated as reject
    result = run_xd(
        ["deploy", "--interactive"],
        cwd=repo,
        home=home,
        stdin="yes\n",
    )
    # Non-TTY = reject = link skipped = failure
    assert result.code != 0
    # FS untouched
    assert not (home / ".a").is_symlink()


def test_interactive_no_skips(run_xd, tmp_repo, unique_home):
    """--interactive with 'no' skips the link, counts as failure."""
    repo = tmp_repo
    home = unique_home
    (repo / "a").write_text("A")
    (repo / "b").write_text("B")
    (repo / "xdotter.toml").write_text('[links]\n"a" = "~/.a"\n"b" = "~/.b"\n')
    (home / ".a").write_text("stale")

    # Yes for a, no for b (but b has no conflict so no prompt)
    result = run_xd(
        ["deploy", "--interactive"],
        cwd=repo,
        home=home,
        stdin="no\n",
    )
    assert result.code != 0
    # .a should NOT be a symlink (rejected)
    assert not (home / ".a").is_symlink()
    # .b should be deployed (no conflict, no prompt needed)
    assert (home / ".b").is_symlink()
