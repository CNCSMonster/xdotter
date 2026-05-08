"""Configuration format per SPEC §"配置格式"."""

import pytest


# ---------------------------------------------------------------
# Empty / minimal config
# ---------------------------------------------------------------

def test_empty_config_is_legal(run_xd, tmp_repo, unique_home):
    """Empty xdotter.toml is legal; deploy/status/undeploy succeed."""
    repo = tmp_repo
    home = unique_home
    (repo / "xdotter.toml").write_text("")

    for cmd in [["deploy"], ["status"], ["undeploy"]]:
        result = run_xd(cmd, cwd=repo, home=home)
        assert result.code == 0, f"{cmd} failed on empty config: {result.stderr}"


def test_only_links_table_empty(run_xd, tmp_repo, unique_home):
    """Only [links] with no entries is legal."""
    repo = tmp_repo
    home = unique_home
    (repo / "xdotter.toml").write_text("[links]\n")
    result = run_xd(["status"], cwd=repo, home=home)
    assert result.code == 0


def test_only_dependencies_table_empty(run_xd, tmp_repo, unique_home):
    """Only [dependencies] with no entries is legal."""
    repo = tmp_repo
    home = unique_home
    (repo / "xdotter.toml").write_text("[dependencies]\n")
    result = run_xd(["status"], cwd=repo, home=home)
    assert result.code == 0


# ---------------------------------------------------------------
# Unknown keys / tables
# ---------------------------------------------------------------

def test_unknown_top_level_key_is_config_error(run_xd, tmp_repo, unique_home):
    """Unknown top-level key is a configuration error."""
    repo = tmp_repo
    home = unique_home
    (repo / "xdotter.toml").write_text("wat = 1\n")
    result = run_xd(["status"], cwd=repo, home=home)
    assert result.code != 0
    assert "[配置错误]" in result.stderr


def test_unknown_top_level_table_is_config_error(run_xd, tmp_repo, unique_home):
    """Unknown top-level table is a configuration error."""
    repo = tmp_repo
    home = unique_home
    (repo / "xdotter.toml").write_text("[unknown]\nx = 1\n")
    result = run_xd(["status"], cwd=repo, home=home)
    assert result.code != 0
    assert "[配置错误]" in result.stderr


# ---------------------------------------------------------------
# TOML syntax errors
# ---------------------------------------------------------------

def test_toml_syntax_error_is_config_error(run_xd, tmp_repo, unique_home):
    """TOML parse error is a configuration error."""
    repo = tmp_repo
    home = unique_home
    (repo / "xdotter.toml").write_text("[links\n")
    result = run_xd(["status"], cwd=repo, home=home)
    assert result.code != 0
    assert "[配置错误]" in result.stderr


# ---------------------------------------------------------------
# Root config must be TOML table
# ---------------------------------------------------------------

def test_root_config_not_a_table(run_xd, tmp_repo, unique_home):
    """Root config that is not a TOML table is a config error."""
    repo = tmp_repo
    home = unique_home
    # A bare string value — not a table
    (repo / "xdotter.toml").write_text('"just a string"\n')
    result = run_xd(["status"], cwd=repo, home=home)
    assert result.code != 0
    assert "[配置错误]" in result.stderr


# ---------------------------------------------------------------
# [links] key/value type validation
# ---------------------------------------------------------------

def test_links_value_not_string_is_config_error(run_xd, tmp_repo, unique_home):
    """[links] value must be a string."""
    repo = tmp_repo
    home = unique_home
    (repo / "xdotter.toml").write_text('[links]\n"a" = 123\n')
    result = run_xd(["status"], cwd=repo, home=home)
    assert result.code != 0
    assert "[配置错误]" in result.stderr


# ---------------------------------------------------------------
# [dependencies] key/value type validation
# ---------------------------------------------------------------

def test_dependencies_value_not_string_is_config_error(run_xd, tmp_repo, unique_home):
    """[dependencies] value must be a string."""
    repo = tmp_repo
    home = unique_home
    (repo / "xdotter.toml").write_text('[dependencies]\n"dep" = 42\n')
    result = run_xd(["status"], cwd=repo, home=home)
    assert result.code != 0
    assert "[配置错误]" in result.stderr


# ---------------------------------------------------------------
# TOML duplicate keys
# ---------------------------------------------------------------

def test_toml_duplicate_keys_rejected(run_xd, tmp_repo, unique_home):
    """TOML duplicate top-level keys are a parse error."""
    repo = tmp_repo
    home = unique_home
    (repo / "xdotter.toml").write_text('[links]\n[links]\n')
    result = run_xd(["status"], cwd=repo, home=home)
    assert result.code != 0
    assert "[配置错误]" in result.stderr
