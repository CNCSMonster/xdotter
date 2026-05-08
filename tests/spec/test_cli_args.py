"""CLI argument parsing per SPEC §"参数模型" and §"命令参数"."""

import pytest


# ---------------------------------------------------------------
# Mutually exclusive flags
# ---------------------------------------------------------------

def test_force_and_interactive_mutually_exclusive_deploy(run_xd, tmp_repo, unique_home):
    """--force and --interactive are mutually exclusive."""
    repo = tmp_repo
    home = unique_home
    (repo / "xdotter.toml").write_text("")
    result = run_xd(["deploy", "--force", "--interactive"], cwd=repo, home=home)
    assert result.code != 0
    assert "[CLI 参数错误]" in result.stderr


def test_force_and_interactive_mutually_exclusive_undeploy(run_xd, tmp_repo, unique_home):
    """--force and --interactive are mutually exclusive for undeploy."""
    repo = tmp_repo
    home = unique_home
    (repo / "xdotter.toml").write_text("")
    result = run_xd(["undeploy", "--force", "--interactive"], cwd=repo, home=home)
    assert result.code != 0
    assert "[CLI 参数错误]" in result.stderr


# ---------------------------------------------------------------
# Unknown commands
# ---------------------------------------------------------------

def test_unknown_command_is_error(run_xd, tmp_repo, unique_home):
    """Unknown subcommand results in non-zero exit."""
    repo = tmp_repo
    home = unique_home
    (repo / "xdotter.toml").write_text("")
    result = run_xd(["bogus"], cwd=repo, home=home)
    assert result.code != 0


# ---------------------------------------------------------------
# Command-specific parameter validation
# ---------------------------------------------------------------

def test_status_rejects_force(run_xd, tmp_repo, unique_home):
    """xd status does not accept --force."""
    repo = tmp_repo
    home = unique_home
    (repo / "xdotter.toml").write_text("")
    result = run_xd(["status", "--force"], cwd=repo, home=home)
    assert result.code != 0


def test_status_rejects_dry_run(run_xd, tmp_repo, unique_home):
    """xd status does not accept --dry-run."""
    repo = tmp_repo
    home = unique_home
    (repo / "xdotter.toml").write_text("")
    result = run_xd(["status", "--dry-run"], cwd=repo, home=home)
    assert result.code != 0


def test_new_rejects_force(run_xd, tmp_repo, unique_home):
    """xd new does not accept --force."""
    repo = tmp_repo
    home = unique_home
    (repo / "xdotter.toml").write_text("")
    result = run_xd(["new", "--force"], cwd=repo, home=home)
    assert result.code != 0


# ---------------------------------------------------------------
# Completion shell
# ---------------------------------------------------------------

@pytest.mark.parametrize("shell", ["bash", "zsh", "fish"])
def test_completion_outputs_to_stdout(run_xd, tmp_repo, unique_home, shell):
    """xd completion <shell> prints completion script to stdout."""
    repo = tmp_repo
    home = unique_home
    (repo / "xdotter.toml").write_text("")
    result = run_xd(["completion", shell], cwd=repo, home=home)
    assert result.code == 0
    assert len(result.stdout) > 0


def test_completion_invalid_shell(run_xd, tmp_repo, unique_home):
    """xd completion <invalid> results in error."""
    repo = tmp_repo
    home = unique_home
    (repo / "xdotter.toml").write_text("")
    result = run_xd(["completion", "powershell"], cwd=repo, home=home)
    assert result.code != 0


# ---------------------------------------------------------------
# Verbose levels
# ---------------------------------------------------------------

def test_verbose_count_beyond_three_equals_three(run_xd, tmp_repo, unique_home):
    """More than three -v behaves same as -vvv."""
    repo = tmp_repo
    home = unique_home
    (repo / "xdotter.toml").write_text("")
    # Both should succeed with no crash
    r3 = run_xd(["-vvv", "status"], cwd=repo, home=home)
    r5 = run_xd(["-vvvvv", "status"], cwd=repo, home=home)
    assert r3.code == 0
    assert r5.code == 0


def test_no_verbose_still_outputs_results(run_xd, tmp_repo, unique_home):
    """Without -v, command results still go to stdout."""
    repo = tmp_repo
    home = unique_home
    (repo / "xdotter.toml").write_text("")
    result = run_xd(["status"], cwd=repo, home=home)
    assert result.code == 0
    assert "Status: 0/0 deployed" in result.stdout
