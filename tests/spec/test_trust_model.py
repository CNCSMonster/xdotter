"""Trust model per SPEC §"配置信任模型"."""


def test_force_does_not_bypass_link_path_safety_checks(run_xd, tmp_repo, unique_home):
    """--force does not bypass link-path safety checks."""
    repo = tmp_repo
    home = unique_home
    (repo / "a").write_text("A")
    (repo / "xdotter.toml").write_text('[links]\n"a" = "~/a/../escape"\n')
    result = run_xd(["deploy", "--force"], cwd=repo, home=home)
    assert result.code != 0
    assert "[配置错误]" in result.stderr


def test_force_does_not_bypass_dependency_containment_checks(run_xd, tmp_repo, unique_home):
    """--force does not bypass dependency containment checks."""
    repo = tmp_repo
    home = unique_home
    (repo / "a").write_text("A")
    (repo / "xdotter.toml").write_text(
        '[links]\n"a" = "~/.a"\n\n[dependencies]\n"esc" = "../escape"\n'
    )
    result = run_xd(["deploy", "--force"], cwd=repo, home=home)
    assert result.code != 0
    assert "[配置错误]" in result.stderr


def test_dry_run_performs_same_safety_validation_as_real_command(run_xd, tmp_repo, unique_home):
    """--dry-run performs same safety validation as real command."""
    repo = tmp_repo
    home = unique_home
    (repo / "xdotter.toml").write_text('[links]\n"ghost" = "~/.ghost"\n')

    real = run_xd(["deploy"], cwd=repo, home=home)
    dry = run_xd(["deploy", "--dry-run"], cwd=repo, home=home)

    # Both should fail with planning error
    assert real.code != 0
    assert dry.code != 0
    assert "[规划阻塞错误]" in real.stderr
    assert "[规划阻塞错误]" in dry.stderr


def test_dry_run_does_not_modify_filesystem(run_xd, tmp_repo, unique_home):
    """--dry-run does not create, delete, or modify any filesystem object."""
    repo = tmp_repo
    home = unique_home
    (repo / "a").write_text("A")
    (repo / "xdotter.toml").write_text('[links]\n"a" = "~/.a"\n')

    result = run_xd(["deploy", "--dry-run"], cwd=repo, home=home)
    assert result.code == 0
    assert not (home / ".a").exists()
    assert not (home / ".a").is_symlink()


def test_new_dry_run_does_not_create_file(run_xd, tmp_repo, unique_home):
    """xd new --dry-run does not create xdotter.toml."""
    repo = tmp_repo
    home = unique_home
    result = run_xd(["new", "--dry-run"], cwd=repo, home=home)
    assert result.code == 0
    assert not (repo / "xdotter.toml").exists()
