"""Permissions per SPEC §"权限和敏感文件语义"."""

import os
import pytest


def _set_mode(path, mode):
    """Set file permissions."""
    os.chmod(path, mode)


def _setup_ssh_deploy(repo, home, key_name="id_test"):
    """Create a source file and deploy to ~/.ssh/<key_name>."""
    ssh_dir = home / ".ssh"
    ssh_dir.mkdir(exist_ok=True)

    src = repo / key_name
    src.write_text("key content")
    # Set overly permissive mode
    _set_mode(src, 0o644)

    (repo / "xdotter.toml").write_text(
        f'[links]\n"{key_name}" = "~/.ssh/{key_name}"\n'
    )
    return src


# ---------------------------------------------------------------
# Permission targets are recognized
# ---------------------------------------------------------------

@pytest.mark.parametrize("link_path", [
    "~/.ssh",
    "~/.ssh/config",
    "~/.ssh/authorized_keys",
    "~/.ssh/id_rsa",
    "~/.ssh/id_ed25519",
    "~/.ssh/id_ecdsa",
    "~/.ssh/id_dsa",
    "~/.ssh/github_rsa",
    "~/.ssh/work_ed25519",
    "~/.pgpass",
    "~/.netrc",
    "~/.gnupg",
])
def test_permission_targets_recognized(link_path, run_xd, tmp_repo, unique_home):
    """All SPEC permission targets trigger permission checks."""
    repo = tmp_repo
    home = unique_home

    if link_path == "~/.ssh" or link_path == "~/.gnupg":
        # Directory target: create a dir source
        src = repo / "srcdir"
        src.mkdir()
        _set_mode(src, 0o755)
        (repo / "xdotter.toml").write_text(
            f'[links]\n"srcdir" = "{link_path}"\n'
        )
    else:
        src = repo / "srcfile"
        src.write_text("content")
        _set_mode(src, 0o644)
        name = link_path.split("/")[-1]
        (repo / "xdotter.toml").write_text(
            f'[links]\n"srcfile" = "{link_path}"\n'
        )

    result = run_xd(["deploy"], cwd=repo, home=home)
    # Permission issue in default mode → skip → non-zero exit
    assert result.code != 0
    # Should mention sensitive target in stderr
    assert "敏感" in result.stderr or "ssh" in result.stderr.lower() or "permission" in result.stderr.lower()


def test_non_permission_target_no_check(run_xd, tmp_repo, unique_home):
    """Path not in SPEC table → no permission check."""
    repo = tmp_repo
    home = unique_home
    src = repo / "bashrc"
    src.write_text("# bash")
    _set_mode(src, 0o644)
    (repo / "xdotter.toml").write_text(
        '[links]\n"bashrc" = "~/.bashrc"\n'
    )
    result = run_xd(["deploy"], cwd=repo, home=home)
    assert result.code == 0, result.stderr
    assert (home / ".bashrc").is_symlink()


def test_stricter_permission_is_acceptable(run_xd, tmp_repo, unique_home):
    """Permission stricter than required → acceptable, no conflict."""
    repo = tmp_repo
    home = unique_home
    ssh_dir = home / ".ssh"
    ssh_dir.mkdir(exist_ok=True)

    src = repo / "id_strict"
    src.write_text("key")
    _set_mode(src, 0o400)  # stricter than 0600

    (repo / "xdotter.toml").write_text(
        '[links]\n"id_strict" = "~/.ssh/id_strict"\n'
    )
    result = run_xd(["deploy"], cwd=repo, home=home)
    assert result.code == 0, result.stderr
    assert (home / ".ssh/id_strict").is_symlink()


def test_force_fixes_permission(run_xd, tmp_repo, unique_home):
    """--force fixes permission on source file."""
    repo = tmp_repo
    home = unique_home
    ssh_dir = home / ".ssh"
    ssh_dir.mkdir(exist_ok=True)

    src = repo / "id_fix"
    src.write_text("key")
    _set_mode(src, 0o644)  # too permissive

    (repo / "xdotter.toml").write_text(
        '[links]\n"id_fix" = "~/.ssh/id_fix"\n'
    )
    result = run_xd(["deploy", "--force"], cwd=repo, home=home)
    assert result.code == 0, result.stderr
    assert (home / ".ssh/id_fix").is_symlink()
    # Source should now have 0600
    actual_mode = os.stat(src).st_mode & 0o777
    assert actual_mode == 0o600


def test_interactive_ask_permission_fix(run_xd, tmp_repo, unique_home):
    """--interactive asks before fixing permission; non-TTY stdin rejects.

    Per SPEC: if stdin is not a TTY, treat as rejection. Since pytest
    always runs with non-TTY stdin, the permission fix is declined.
    """
    repo = tmp_repo
    home = unique_home
    ssh_dir = home / ".ssh"
    ssh_dir.mkdir(exist_ok=True)

    src = repo / "id_interactive"
    src.write_text("key")
    _set_mode(src, 0o644)

    (repo / "xdotter.toml").write_text(
        '[links]\n"id_interactive" = "~/.ssh/id_interactive"\n'
    )
    result = run_xd(
        ["deploy", "--interactive"],
        cwd=repo,
        home=home,
        stdin="yes\n",
    )
    # Non-TTY = reject = permission fix declined = failure
    assert result.code != 0
    # FS untouched — source mode unchanged
    actual_mode = os.stat(src).st_mode & 0o777
    assert actual_mode == 0o644


def test_dry_run_reports_permission_fix(run_xd, tmp_repo, unique_home):
    """--dry-run reports permission fix without applying it."""
    repo = tmp_repo
    home = unique_home
    ssh_dir = home / ".ssh"
    ssh_dir.mkdir(exist_ok=True)

    src = repo / "id_dry"
    src.write_text("key")
    _set_mode(src, 0o644)

    (repo / "xdotter.toml").write_text(
        '[links]\n"id_dry" = "~/.ssh/id_dry"\n'
    )
    result = run_xd(["deploy", "--force", "--dry-run"], cwd=repo, home=home)
    assert result.code == 0
    assert "perm" in result.stdout.lower() or "权限" in result.stdout
    # FS untouched — source mode unchanged
    actual_mode = os.stat(src).st_mode & 0o777
    assert actual_mode == 0o644
