"""Exit codes per SPEC §"退出码"."""


def test_success_is_zero(run_xd, tmp_repo, unique_home):
    """Successful command exits 0."""
    repo = tmp_repo
    home = unique_home
    (repo / "xdotter.toml").write_text("")
    result = run_xd(["status"], cwd=repo, home=home)
    assert result.code == 0


def test_failure_is_nonzero(run_xd, tmp_repo, unique_home):
    """Failed command exits non-zero."""
    repo = tmp_repo
    home = unique_home
    (repo / "xdotter.toml").write_text("invalid = toml\n[")
    result = run_xd(["status"], cwd=repo, home=home)
    assert result.code != 0


def test_exit_code_not_zero_means_failure_not_specific_value(run_xd, tmp_repo, unique_home):
    """Verify: only != 0 matters, not the specific value.

    Different errors may use different non-zero values.
    """
    repo = tmp_repo
    home = unique_home

    # Different error types
    configs = [
        "unknown = 1\n",       # config error
        '[links]\n"ghost" = "~/.ghost"\n',  # planning error
    ]

    codes = set()
    for content in configs:
        (repo / "xdotter.toml").write_text(content)
        result = run_xd(["deploy"], cwd=repo, home=home)
        assert result.code != 0
        codes.add(result.code)

    # The specific values may differ; what matters is all are non-zero
    for code in codes:
        assert code != 0
