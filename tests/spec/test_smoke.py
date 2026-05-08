"""Smoke test to verify the test harness works."""

import pytest


def test_smoke_infra_works(run_xd, tmp_repo, unique_home, xd_binary):
    """Verify binary resolves and a minimal command works."""
    assert xd_binary.exists(), f"binary not found at {xd_binary}"

    repo = tmp_repo
    home = unique_home
    (repo / "xdotter.toml").write_text("")

    result = run_xd(["version"], cwd=repo, home=home)
    assert result.code == 0
    assert "xdotter" in result.stdout
    assert result.stderr == ""
