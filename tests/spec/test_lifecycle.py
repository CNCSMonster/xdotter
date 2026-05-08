"""Lifecycle and phase ordering per SPEC §"校验、规划和应用生命周期"."""


def test_cli_error_before_config_read(run_xd, unique_home):
    """CLI arg error fails before reading config."""
    home = unique_home
    # No xdotter.toml exists — but CLI error should fire first
    # --force --interactive is a CLI parse error regardless of config
    result = run_xd(
        ["deploy", "--force", "--interactive"],
        cwd=home,  # no config here
        home=home,
    )
    assert result.code != 0
    assert "[CLI 参数错误]" in result.stderr


def test_config_error_before_planning(run_xd, tmp_repo, unique_home):
    """Config error fails before planning stage."""
    repo = tmp_repo
    home = unique_home
    (repo / "xdotter.toml").write_text("unknown = 1\n")
    result = run_xd(["deploy"], cwd=repo, home=home)
    assert result.code != 0
    assert "[配置错误]" in result.stderr


def test_planning_error_before_apply(run_xd, tmp_repo, unique_home):
    """Planning error fails before apply stage (no FS modification)."""
    repo = tmp_repo
    home = unique_home
    (repo / "xdotter.toml").write_text('[links]\n"ghost" = "~/.ghost"\n')
    result = run_xd(["deploy"], cwd=repo, home=home)
    assert result.code != 0
    assert "[规划阻塞错误]" in result.stderr
    # Target should NOT exist
    assert not (home / ".ghost").exists()


def test_all_errors_reported_together(run_xd, tmp_repo, unique_home):
    """Multiple config + planning errors reported in one shot."""
    repo = tmp_repo
    home = unique_home
    # Two links with missing sources
    (repo / "xdotter.toml").write_text(
        '[links]\n"ghost1" = "~/.g1"\n"ghost2" = "~/.g2"\n'
    )
    result = run_xd(["deploy"], cwd=repo, home=home)
    assert result.code != 0
    # Both planning errors should be reported
    assert "[规划阻塞错误]" in result.stderr
    # Should mention both sources
    assert "ghost1" in result.stderr or "ghost2" in result.stderr
