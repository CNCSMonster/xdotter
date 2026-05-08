"""Error classification per SPEC §"错误分类"."""


# ---------------------------------------------------------------
# All four error classes are reachable
# ---------------------------------------------------------------

def test_cli_error_label(run_xd, tmp_repo, unique_home):
    """CLI argument error → [CLI 参数错误]."""
    repo = tmp_repo
    home = unique_home
    (repo / "xdotter.toml").write_text("")
    result = run_xd(["deploy", "--force", "--interactive"], cwd=repo, home=home)
    assert result.code != 0
    assert "[CLI 参数错误]" in result.stderr


def test_config_error_label(run_xd, tmp_repo, unique_home):
    """Configuration error → [配置错误]."""
    repo = tmp_repo
    home = unique_home
    (repo / "xdotter.toml").write_text("unknown = 1\n")
    result = run_xd(["status"], cwd=repo, home=home)
    assert result.code != 0
    assert "[配置错误]" in result.stderr


def test_planning_error_label(run_xd, tmp_repo, unique_home):
    """Planning-block error → [规划阻塞错误]."""
    repo = tmp_repo
    home = unique_home
    (repo / "xdotter.toml").write_text('[links]\n"ghost" = "~/.ghost"\n')
    result = run_xd(["deploy"], cwd=repo, home=home)
    assert result.code != 0
    assert "[规划阻塞错误]" in result.stderr


def test_undeploy_non_symlink_warning_has_error_label(run_xd, tmp_repo, unique_home):
    """Non-symlink at link path during undeploy → warning + failure."""
    repo = tmp_repo
    home = unique_home
    (repo / "a").write_text("x")
    (home / ".target").write_text("real file")
    (repo / "xdotter.toml").write_text('[links]\n"a" = "~/.target"\n')
    # Deploy first
    run_xd(["deploy"], cwd=repo, home=home)
    # Now undeploy — link path is a real file, not a symlink
    result = run_xd(["undeploy"], cwd=repo, home=home)
    assert result.code != 0
    # The error should be at planning/apply level — it's a recoverable conflict
    # skipped in default mode
    assert "[规划阻塞错误]" in result.stderr or "planning" in result.stderr.lower()


# ---------------------------------------------------------------
# Error messages contain classifiable labels + path info
# ---------------------------------------------------------------

def test_config_error_includes_path_info(run_xd, tmp_repo, unique_home):
    """Config error includes relevant path information."""
    repo = tmp_repo
    home = unique_home
    (repo / "a").write_text("x")
    (repo / "xdotter.toml").write_text('[links]\n"a" = "relative"\n')
    result = run_xd(["deploy"], cwd=repo, home=home)
    assert result.code != 0
    assert "[配置错误]" in result.stderr
    # Should mention the link path or source path
    assert "a" in result.stderr or "relative" in result.stderr


def test_planning_error_includes_source_path(run_xd, tmp_repo, unique_home):
    """Planning error includes source path reference."""
    repo = tmp_repo
    home = unique_home
    (repo / "xdotter.toml").write_text('[links]\n"missing" = "~/.missing"\n')
    result = run_xd(["deploy"], cwd=repo, home=home)
    assert result.code != 0
    assert "[规划阻塞错误]" in result.stderr
    assert "missing" in result.stderr


# ---------------------------------------------------------------
# --force does NOT repair config/planning errors
# ---------------------------------------------------------------

def test_force_does_not_repair_config_error(run_xd, tmp_repo, unique_home):
    """--force does not make illegal config legal."""
    repo = tmp_repo
    home = unique_home
    (repo / "a").write_text("x")
    (repo / "xdotter.toml").write_text('[links]\n"a" = "relative/path"\n')
    result = run_xd(["deploy", "--force"], cwd=repo, home=home)
    assert result.code != 0
    assert "[配置错误]" in result.stderr


def test_force_does_not_repair_planning_error(run_xd, tmp_repo, unique_home):
    """--force does not make missing source legal."""
    repo = tmp_repo
    home = unique_home
    (repo / "xdotter.toml").write_text('[links]\n"ghost" = "~/.ghost"\n')
    result = run_xd(["deploy", "--force"], cwd=repo, home=home)
    assert result.code != 0
    assert "[规划阻塞错误]" in result.stderr


def test_interactive_does_not_repair_config_error(run_xd, tmp_repo, unique_home):
    """--interactive does not make illegal config legal."""
    repo = tmp_repo
    home = unique_home
    (repo / "a").write_text("x")
    (repo / "xdotter.toml").write_text('[links]\n"a" = "/root/../etc"\n')
    result = run_xd(["deploy", "--interactive"], cwd=repo, home=home)
    assert result.code != 0
    assert "[配置错误]" in result.stderr
