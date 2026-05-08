"""Output semantics per SPEC §"输出语义"."""


# ---------------------------------------------------------------
# stdout vs stderr separation
# ---------------------------------------------------------------

def test_command_result_to_stdout(run_xd, tmp_repo, unique_home):
    """Command results go to stdout."""
    repo = tmp_repo
    home = unique_home
    (repo / "xdotter.toml").write_text("")
    result = run_xd(["status"], cwd=repo, home=home)
    assert "Status: 0/0 deployed" in result.stdout


def test_warnings_and_errors_to_stderr(run_xd, tmp_repo, unique_home):
    """Warnings and errors go to stderr, not stdout."""
    repo = tmp_repo
    home = unique_home
    (repo / "a").write_text("A")
    (repo / "xdotter.toml").write_text('[links]\n"a" = "relative"\n')
    result = run_xd(["deploy"], cwd=repo, home=home)
    assert result.code != 0
    assert "[配置错误]" in result.stderr
    # Error should NOT appear in stdout
    assert "[配置错误]" not in result.stdout


def test_successful_deploy_stderr_has_no_error_labels(run_xd, tmp_repo, unique_home):
    """Successful deploy with no conflicts/warnings → stderr has no errors."""
    repo = tmp_repo
    home = unique_home
    (repo / "a").write_text("A")
    (repo / "xdotter.toml").write_text('[links]\n"a" = "~/.a"\n')
    result = run_xd(["deploy"], cwd=repo, home=home)
    assert result.code == 0
    # No errors on stderr (may have info/diagnostic output)
    assert "[配置错误]" not in result.stderr
    assert "[规划阻塞错误]" not in result.stderr
    assert "[应用阶段错误]" not in result.stderr


def test_sensitive_target_warning_to_stderr(run_xd, tmp_repo, unique_home):
    """Sensitive target warning goes to stderr even when deploy succeeds."""
    repo = tmp_repo
    home = unique_home
    ssh_dir = home / ".ssh"
    ssh_dir.mkdir(exist_ok=True)

    src = repo / "id_test"
    src.write_text("key")
    import os
    os.chmod(src, 0o600)  # correct perms, no conflict

    (repo / "xdotter.toml").write_text(
        '[links]\n"id_test" = "~/.ssh/id_test"\n'
    )
    result = run_xd(["deploy"], cwd=repo, home=home)
    assert result.code == 0  # succeeds (permission is ok)
    # But warning about sensitive target still appears on stderr
    assert "敏感" in result.stderr or "ssh" in result.stderr.lower()


# ---------------------------------------------------------------
# Verbose levels on deploy (not just status)
# ---------------------------------------------------------------

def test_verbose_deploy_shows_operation_info(run_xd, tmp_repo, unique_home):
    """-v on deploy shows operation info to stderr."""
    repo = tmp_repo
    home = unique_home
    (repo / "a").write_text("A")
    (repo / "xdotter.toml").write_text('[links]\n"a" = "~/.a"\n')
    result = run_xd(["-v", "deploy"], cwd=repo, home=home)
    assert result.code == 0
    # Verbose diagnostics on stderr
    assert len(result.stderr) > 0


def test_verbose_status_shows_correct_links(run_xd, tmp_repo, unique_home):
    """status -v shows correct link paths."""
    repo = tmp_repo
    home = unique_home
    (repo / "a").write_text("A")
    (repo / "xdotter.toml").write_text('[links]\n"a" = "~/.a"\n')
    run_xd(["deploy"], cwd=repo, home=home)

    result = run_xd(["-v", "status"], cwd=repo, home=home)
    assert result.code == 0
    assert ".a" in result.stdout
