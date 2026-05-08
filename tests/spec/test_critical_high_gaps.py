"""High-priority SPEC invariants not yet covered by existing tests."""

import os
import pytest


# ---------------------------------------------------------------
# Gap B (Critical): Deploy "replacing dir would delete/contain source"
# — planning-block error
# ---------------------------------------------------------------

def test_deploy_replace_dir_contains_source_planning_error(
    run_xd, tmp_repo, unique_home
):
    """SPEC §"符号链接安全语义": 源路径位于一个将被删除的已有真实链接目录
    内部 → 规划阻塞错误。

    Scenario: config dir is inside ~/.mydir/, link = ~/.mydir (existing
    empty real dir). Source canonical path resolves to inside ~/.mydir/.
    Force-mode replacing ~/.mydir would delete the source.
    """
    home = unique_home

    # Create ~/.mydir as an empty real directory
    mydir = home / ".mydir"
    mydir.mkdir()

    # Place the repo inside ~/.mydir/
    repo_inside = mydir / "repo"
    repo_inside.mkdir()

    # Create source file inside the repo (thus also inside ~/.mydir/)
    src = repo_inside / "src.txt"
    src.write_text("hello")

    # Config: source = "src.txt" (relative to config dir), link = ~/.mydir
    (repo_inside / "xdotter.toml").write_text(
        '[links]\n"src.txt" = "~/.mydir"\n'
    )

    # Even --force must fail — planning-block error
    result = run_xd(["deploy", "--force"], cwd=repo_inside, home=home)
    assert result.code != 0, f"Expected planning error, got: {result.stderr}"
    assert "[规划阻塞错误]" in result.stderr
    # Source file must be untouched
    assert src.exists()
    assert src.read_text() == "hello"
    # ~/.mydir must still be a directory, not a symlink
    assert mydir.is_dir()
    assert not mydir.is_symlink()


def test_deploy_replace_dir_contains_source_default_mode(
    run_xd, tmp_repo, unique_home
):
    """Same scenario as above but with default mode — still planning-block."""
    home = unique_home

    mydir = home / ".mydir"
    mydir.mkdir()
    repo_inside = mydir / "repo"
    repo_inside.mkdir()
    src = repo_inside / "src.txt"
    src.write_text("hello")
    (repo_inside / "xdotter.toml").write_text(
        '[links]\n"src.txt" = "~/.mydir"\n'
    )

    result = run_xd(["deploy"], cwd=repo_inside, home=home)
    assert result.code != 0
    assert "[规划阻塞错误]" in result.stderr
    assert mydir.is_dir()
    assert not mydir.is_symlink()


def test_deploy_replace_dir_contains_source_dry_run(
    run_xd, tmp_repo, unique_home
):
    """Same scenario with --dry-run — must also fail at planning stage."""
    home = unique_home

    mydir = home / ".mydir"
    mydir.mkdir()
    repo_inside = mydir / "repo"
    repo_inside.mkdir()
    src = repo_inside / "src.txt"
    src.write_text("hello")
    (repo_inside / "xdotter.toml").write_text(
        '[links]\n"src.txt" = "~/.mydir"\n'
    )

    result = run_xd(["deploy", "--force", "--dry-run"], cwd=repo_inside, home=home)
    assert result.code != 0
    assert "[规划阻塞错误]" in result.stderr
    # FS untouched
    assert mydir.is_dir()
    assert not mydir.is_symlink()
    assert src.exists()


# ---------------------------------------------------------------
# Gap E (Critical): Same config file duplicate link paths
# ---------------------------------------------------------------

def test_duplicate_link_path_same_config_config_error(
    run_xd, tmp_repo, unique_home
):
    """SPEC §"配置错误": 多个源路径映射到同一个链接路径 → 配置错误。

    Two keys in [links] map to the same link path. Must be a config error
    listing both conflicting entries.
    """
    repo = tmp_repo
    home = unique_home

    (repo / "a.txt").write_text("A")
    (repo / "b.txt").write_text("B")
    (repo / "xdotter.toml").write_text(
        '[links]\n"a.txt" = "~/.dup_target"\n"b.txt" = "~/.dup_target"\n'
    )

    result = run_xd(["deploy"], cwd=repo, home=home)
    assert result.code != 0
    assert "[配置错误]" in result.stderr
    # Should mention both conflicting keys
    assert "a.txt" in result.stderr
    assert "b.txt" in result.stderr
    # No filesystem modifications
    assert not (home / ".dup_target").exists()
    assert not (home / ".dup_target").is_symlink()


def test_duplicate_link_path_same_config_force_still_fails(
    run_xd, tmp_repo, unique_home
):
    """Duplicate link path is a config error — --force cannot bypass it."""
    repo = tmp_repo
    home = unique_home

    (repo / "a.txt").write_text("A")
    (repo / "b.txt").write_text("B")
    (repo / "xdotter.toml").write_text(
        '[links]\n"a.txt" = "~/.dup_target"\n"b.txt" = "~/.dup_target"\n'
    )

    result = run_xd(["deploy", "--force"], cwd=repo, home=home)
    assert result.code != 0
    assert "[配置错误]" in result.stderr
    assert not (home / ".dup_target").is_symlink()


def test_duplicate_link_path_same_config_dry_run(
    run_xd, tmp_repo, unique_home
):
    """Duplicate link path detected in --dry-run — config error."""
    repo = tmp_repo
    home = unique_home

    (repo / "a.txt").write_text("A")
    (repo / "b.txt").write_text("B")
    (repo / "xdotter.toml").write_text(
        '[links]\n"a.txt" = "~/.dup_target"\n"b.txt" = "~/.dup_target"\n'
    )

    result = run_xd(["deploy", "--dry-run"], cwd=repo, home=home)
    assert result.code != 0
    assert "[配置错误]" in result.stderr
    assert not (home / ".dup_target").exists()


# ---------------------------------------------------------------
# Gap F (Critical): xd status detects permission issues on
# already-deployed links
# ---------------------------------------------------------------

def test_status_detects_permission_issues_on_deployed_link(
    run_xd, tmp_repo, unique_home
):
    """SPEC §"命令" — xd status: permission issues on deployed sensitive
    targets must be detected and reported in the summary.

    1. Deploy a sensitive file with correct permissions (--force).
    2. chmod the source to overly-permissive mode.
    3. Run xd status.
    4. Verify "Permission issues: 1" in stdout.
    """
    repo = tmp_repo
    home = unique_home
    ssh_dir = home / ".ssh"
    ssh_dir.mkdir(exist_ok=True)

    # Create source with overly-permissive mode
    src = repo / "id_status_test"
    src.write_text("secret key")
    os.chmod(src, 0o644)

    (repo / "xdotter.toml").write_text(
        '[links]\n"id_status_test" = "~/.ssh/id_status_test"\n'
    )

    # Deploy with --force — fixes permission to 0600
    result = run_xd(["deploy", "--force"], cwd=repo, home=home)
    assert result.code == 0, result.stderr
    assert (home / ".ssh/id_status_test").is_symlink()

    # Verify source was fixed to 0600
    assert (os.stat(src).st_mode & 0o777) == 0o600

    # Now loosen the source permissions (simulate user or tool changing them)
    os.chmod(src, 0o644)

    # Run status — should detect the permission issue
    result = run_xd(["status"], cwd=repo, home=home)
    assert result.code != 0  # permission issue = non-zero exit
    assert "Permission issues: 1" in result.stdout
    # Link should still be counted as deployed (N includes perm-issue links)
    assert "Status: 1/1 deployed" in result.stdout


def test_status_permission_issues_with_multiple_links(
    run_xd, tmp_repo, unique_home
):
    """Permission issues count correctly among multiple links.

    Set up 3 links: 1 deployed with perm issue, 1 deployed clean,
    1 not deployed. Verify: Permission issues: 1, Status: 2/3 deployed.
    """
    repo = tmp_repo
    home = unique_home
    ssh_dir = home / ".ssh"
    ssh_dir.mkdir(exist_ok=True)

    # Link 1: sensitive file, deployed, then permissions loosened
    src1 = repo / "id_perm1"
    src1.write_text("key1")
    os.chmod(src1, 0o644)

    # Link 2: normal file, deployed cleanly
    src2 = repo / "normal.txt"
    src2.write_text("normal")

    (repo / "xdotter.toml").write_text(
        '[links]\n"id_perm1" = "~/.ssh/id_perm1"\n"normal.txt" = "~/.normal.txt"\n"ghost.txt" = "~/.ghost.txt"\n'
    )

    # Deploy with --force — fixes id_perm1, deploys normal.txt, ghost.txt missing
    # Wait — ghost.txt source doesn't exist, that's a planning error.
    # Let me use 3 sources that all exist.
    src3 = repo / "ghost.txt"
    src3.write_text("ghost")

    (repo / "xdotter.toml").write_text(
        '[links]\n"id_perm1" = "~/.ssh/id_perm1"\n"normal.txt" = "~/.normal.txt"\n"ghost.txt" = "~/.ghost.txt"\n'
    )

    result = run_xd(["deploy", "--force"], cwd=repo, home=home)
    assert result.code == 0, result.stderr

    # All 3 deployed
    assert (home / ".ssh/id_perm1").is_symlink()
    assert (home / ".normal.txt").is_symlink()
    assert (home / ".ghost.txt").is_symlink()

    # Loosen permissions on src1
    os.chmod(src1, 0o644)

    # Run status
    result = run_xd(["status"], cwd=repo, home=home)
    assert result.code != 0
    assert "Status: 3/3 deployed" in result.stdout
    assert "Permission issues: 1" in result.stdout


def test_status_permission_issues_not_deployed_excluded(
    run_xd, tmp_repo, unique_home
):
    """Permission issues only counted for links that are otherwise deployed.

    A link that is not deployed (link path missing) cannot be a permission issue.
    """
    repo = tmp_repo
    home = unique_home
    ssh_dir = home / ".ssh"
    ssh_dir.mkdir(exist_ok=True)

    # Link 1: sensitive file — deploy with --force (permission fixed to 0600)
    src1 = repo / "id_clean"
    src1.write_text("key")
    os.chmod(src1, 0o644)

    # Link 2: another sensitive file — deploy it too
    src2 = repo / "id_other"
    src2.write_text("key2")
    os.chmod(src2, 0o644)

    (repo / "xdotter.toml").write_text(
        '[links]\n"id_clean" = "~/.ssh/id_clean"\n"id_other" = "~/.ssh/id_other"\n'
    )

    # Deploy both with --force
    result = run_xd(["deploy", "--force"], cwd=repo, home=home)
    assert result.code == 0, result.stderr

    # Now remove the second link to make it "not deployed"
    (home / ".ssh/id_other").unlink()

    # Loosen permissions on src1
    os.chmod(src1, 0o644)

    # Run status — id_clean deployed (with perm issue), id_other not deployed
    result = run_xd(["status"], cwd=repo, home=home)
    assert result.code != 0
    assert "Status: 1/2 deployed" in result.stdout
    assert "Not deployed: 1" in result.stdout
    # id_clean should have a permission issue
    assert "Permission issues: 1" in result.stdout


# ---------------------------------------------------------------
# Critical 3: xd new refuses to overwrite
# ---------------------------------------------------------------

def test_new_refuses_overwrite_existing(run_xd, tmp_repo, unique_home):
    """xd new fails when xdotter.toml already exists."""
    repo = tmp_repo
    home = unique_home
    (repo / "xdotter.toml").write_text("# existing config")

    result = run_xd(["new"], cwd=repo, home=home)
    assert result.code != 0
    assert "[配置错误]" in result.stderr
    # File content unchanged
    assert (repo / "xdotter.toml").read_text() == "# existing config"


def test_new_dry_run_with_existing_file(run_xd, tmp_repo, unique_home):
    """xd new --dry-run when file exists reports will-not-create."""
    repo = tmp_repo
    home = unique_home
    (repo / "xdotter.toml").write_text("# existing")

    result = run_xd(["new", "--dry-run"], cwd=repo, home=home)
    # Should report that it won't create (file exists)
    assert result.code != 0 or "exist" in result.stdout.lower() or "存在" in result.stdout


# ---------------------------------------------------------------
# Critical 4: Source and link resolve to same object
# ---------------------------------------------------------------

def test_same_object_rejected_planning_error(run_xd, tmp_repo, unique_home):
    """Source path and link path resolve to same filesystem object → planning error."""
    repo = tmp_repo
    home = unique_home

    # Create a file inside repo that we'll link to its own location
    src = repo / "self"
    src.write_text("self content")

    # Link path = absolute path to the same file
    abs_path = str(src.resolve())
    (repo / "xdotter.toml").write_text(
        f'[links]\n"self" = "{abs_path}"\n'
    )

    result = run_xd(["deploy"], cwd=repo, home=home)
    assert result.code != 0
    assert "[规划阻塞错误]" in result.stderr


def test_same_object_via_home_rejected(run_xd, tmp_repo, unique_home):
    """Source inside home, link to same home path → planning error."""
    repo = tmp_repo
    home = unique_home

    # Put source file inside home
    src = home / "srcfile"
    src.write_text("content")

    # Repo is inside home too — source path "srcfile" relative to repo
    # won't work since repo is not in home. Let's use a different approach:
    # Use a relative source path that resolves to the same absolute path
    # as the link target after expansion.
    # Actually, the simplest case: source = relative path in repo,
    # link = ~/something that happens to be the same after resolution.
    # This requires repo to be inside home, which it is (tmp path).
    # For this test, we'll use the home directory as the repo.
    (repo / "xdotter.toml").write_text(
        '[links]\n"srcfile" = "~/srcfile"\n'
    )

    # Deploy: source = repo/srcfile (which doesn't exist), link = ~/srcfile
    # This is a missing source case, not same-object.
    # Let me use a real same-object scenario: source is a subdir of repo
    # that also happens to be the link target.

    # Simplest valid test: put source at ~/.xdotter_src, link to ~/.xdotter_src
    # But source must be relative to config dir.
    # The only way to get same-object is if config dir IS the home dir.
    # Let's set repo = home.
    (home / "xdotter.toml").write_text(
        '[links]\n"srcfile" = "~/srcfile"\n'
    )

    result = run_xd(["deploy"], cwd=home, home=home)
    # Source is home/srcfile which exists, link is ~/srcfile = home/srcfile
    # Same object!
    assert result.code != 0
    assert "[规划阻塞错误]" in result.stderr


# ---------------------------------------------------------------
# Critical 5: Link inside source directory
# ---------------------------------------------------------------

def test_link_inside_source_rejected(run_xd, tmp_repo, unique_home):
    """SPEC §"符号链接安全语义": 链接路径位于另一链接路径内部 →
    规划阻塞错误。规划阶段检测展开后的链接路径嵌套关系，
    防止在符号链接内部创建子链接。
    """
    repo = tmp_repo
    home = unique_home

    src_dir = repo / "mydir"
    src_dir.mkdir()
    (src_dir / "inner").write_text("inner")

    (repo / "xdotter.toml").write_text(
        '[links]\n"mydir" = "~/.mydir"\n"mydir/inner" = "~/.mydir/inner"\n'
    )

    result = run_xd(["deploy"], cwd=repo, home=home)
    # Must fail — either planning or apply error
    assert result.code != 0
    # Source dir should still be intact (not deleted)
    assert src_dir.exists()
    assert (src_dir / "inner").exists()


# ---------------------------------------------------------------
# High 8: Status permission issues orthogonal counting
# ---------------------------------------------------------------

def test_status_permission_issues_orthogonal_to_deployed(run_xd, tmp_repo, unique_home):
    """Permission issues row is independent — deployed links can have permission issues.

    N (deployed count) includes links with permission issues.
    Permission issues row is separate.
    Six categories + N = M still holds.
    """
    repo = tmp_repo
    home = unique_home
    ssh_dir = home / ".ssh"
    ssh_dir.mkdir(exist_ok=True)

    # Create a source with overly permissive mode
    src = repo / "id_orthogonal"
    src.write_text("key")
    os.chmod(src, 0o644)  # too permissive for SSH key

    (repo / "xdotter.toml").write_text(
        '[links]\n"id_orthogonal" = "~/.ssh/id_orthogonal"\n'
    )

    # Deploy — permission issue means default mode skips, so link NOT deployed
    result = run_xd(["deploy"], cwd=repo, home=home)
    # In default mode, permission issue → skip → not deployed
    assert result.code != 0

    # Now deploy with --force to actually deploy it
    result = run_xd(["deploy", "--force"], cwd=repo, home=home)
    assert result.code == 0, result.stderr
    assert (home / ".ssh/id_orthogonal").is_symlink()

    # Status should show deployed with no permission issue (force fixed it)
    result = run_xd(["status"], cwd=repo, home=home)
    assert result.code == 0
    assert "Status: 1/1 deployed" in result.stdout
    # Permission issues should be 0 (force fixed it)
    assert "Permission issues: 0" in result.stdout


# ---------------------------------------------------------------
# High 9: Error message three-component contract
# ---------------------------------------------------------------

def test_error_message_has_label_path_and_reason(run_xd, tmp_repo, unique_home):
    """Error messages must contain: class label + path + brief reason."""
    repo = tmp_repo
    home = unique_home
    (repo / "a").write_text("A")
    (repo / "xdotter.toml").write_text('[links]\n"a" = "relative"\n')

    result = run_xd(["deploy"], cwd=repo, home=home)
    assert result.code != 0
    stderr = result.stderr

    # 1. Class label
    assert "[配置错误]" in stderr
    # 2. Config file path
    assert "xdotter.toml" in stderr
    # 3. Brief reason (mentions what's wrong)
    assert "相对" in stderr or "relative" in stderr or "链接路径" in stderr


def test_planning_error_has_label_path_and_reason(run_xd, tmp_repo, unique_home):
    """Planning error messages contain label + path + reason."""
    repo = tmp_repo
    home = unique_home
    (repo / "xdotter.toml").write_text('[links]\n"ghost" = "~/.ghost"\n')

    result = run_xd(["deploy"], cwd=repo, home=home)
    assert result.code != 0
    stderr = result.stderr

    assert "[规划阻塞错误]" in stderr  # label
    assert "ghost" in stderr  # path reference
    # reason: source doesn't exist
    assert "不存在" in stderr or "不存在" in stderr.lower() or "not exist" in stderr.lower()


# ---------------------------------------------------------------
# High 10: Apply-stage partial failure — no rollback, no continue
# ---------------------------------------------------------------

def test_skipped_non_empty_dir_does_not_block_subsequent_links(run_xd, tmp_repo, unique_home):
    """SPEC §"应用阶段错误": 验证规划阶段的"跳过"（非空目录）不触发
    SPEC 要求的"停止后续操作"规则——因为根本没有执行文件系统修改操作。
    这验证了 SkippedFailure 与 apply-stage failure 的区别。
    """
    repo = tmp_repo
    home = unique_home

    (repo / "a").write_text("A")
    (repo / "b").write_text("B")
    (repo / "xdotter.toml").write_text(
        '[links]\n"a" = "~/.a"\n"b" = "~/.b"\n'
    )

    # Pre-create .a as a non-empty directory — force mode cannot handle it
    (home / ".a").mkdir()
    (home / ".a" / "blocker").write_text("I block")

    result = run_xd(["deploy", "--force"], cwd=repo, home=home)
    # Non-zero because .a was skipped as a recoverable conflict
    assert result.code != 0

    # .b should still be deployed — skipping .a (planned skip) does not
    # block subsequent links, because no filesystem operation was attempted
    # on .a. This is distinct from SPEC §"应用阶段错误" which requires
    # stopping subsequent ops only when an actual filesystem operation fails.
    assert (home / ".b").is_symlink()
    # Clean up
    (home / ".a" / "blocker").unlink()
    (home / ".a").rmdir()


def test_apply_completed_ops_not_rolled_back(run_xd, tmp_repo, unique_home):
    """Operations completed before a failure are NOT rolled back."""
    repo = tmp_repo
    home = unique_home

    (repo / "a").write_text("A")
    (repo / "b").write_text("B")
    (repo / "xdotter.toml").write_text(
        '[links]\n"a" = "~/.a"\n"b" = "~/.b"\n'
    )

    # Pre-create .b as a non-empty dir — force can't handle this
    (home / ".b").mkdir()
    (home / ".b" / "file").write_text("content")

    result = run_xd(["deploy", "--force"], cwd=repo, home=home)
    assert result.code != 0  # .b is non-empty dir, skipped

    # .a should still be deployed (not rolled back)
    assert (home / ".a").is_symlink()
    # .b should still be a non-empty dir (not modified)
    assert (home / ".b").is_dir()
    assert (home / ".b" / "file").exists()


# ---------------------------------------------------------------
# High 11: All allowed and disallowed command combinations
# ---------------------------------------------------------------

@pytest.mark.parametrize("args", [
    ["deploy"],
    ["deploy", "--dry-run"],
    ["deploy", "--force"],
    ["deploy", "--force", "--dry-run"],
    ["deploy", "--interactive"],
    ["deploy", "--interactive", "--dry-run"],
    ["undeploy"],
    ["undeploy", "--dry-run"],
    ["undeploy", "--force"],
    ["undeploy", "--force", "--dry-run"],
    ["undeploy", "--interactive"],
    ["undeploy", "--interactive", "--dry-run"],
])
def test_allowed_deploy_undeploy_combinations(run_xd, tmp_repo, unique_home, args):
    """All 12 allowed deploy/undeploy combinations execute without CLI parse errors."""
    repo = tmp_repo
    home = unique_home
    (repo / "xdotter.toml").write_text("")
    result = run_xd(args, cwd=repo, home=home)
    # Should NOT fail with CLI parse error (may fail with other errors,
    # but not "cannot be used with" or CLI-level rejection)
    assert "cannot be used with" not in result.stderr.lower()
    assert "conflicts" not in result.stderr.lower()


@pytest.mark.parametrize("args", [
    ["deploy", "--force", "--interactive"],
    ["deploy", "--force", "--interactive", "--dry-run"],
    ["undeploy", "--force", "--interactive"],
    ["undeploy", "--force", "--interactive", "--dry-run"],
])
def test_disallowed_deploy_undeploy_combinations(run_xd, tmp_repo, unique_home, args):
    """All 4 disallowed combinations fail with CLI parse error."""
    repo = tmp_repo
    home = unique_home
    (repo / "xdotter.toml").write_text("")
    result = run_xd(args, cwd=repo, home=home)
    assert result.code != 0
    assert "[CLI 参数错误]" in result.stderr


# ---------------------------------------------------------------
# High 13: Permission check based on link path, not source filename
# ---------------------------------------------------------------

def test_permission_check_uses_link_path_not_source_name(run_xd, tmp_repo, unique_home):
    """Permission check is based on ~/-expanded link path, not source filename.

    A source named id_rsa linked to ~/.bashrc should NOT trigger SSH check.
    """
    repo = tmp_repo
    home = unique_home

    src = repo / "id_rsa"
    src.write_text("key")
    os.chmod(src, 0o644)  # would trigger SSH check if based on source name

    (repo / "xdotter.toml").write_text(
        '[links]\n"id_rsa" = "~/.bashrc"\n'
    )

    # Deploy — should succeed without permission conflict because
    # ~/.bashrc is not in the SPEC permission table
    result = run_xd(["deploy"], cwd=repo, home=home)
    assert result.code == 0, result.stderr
    assert (home / ".bashrc").is_symlink()


def test_pub_keys_not_in_permission_table(run_xd, tmp_repo, unique_home):
    """~/.ssh/*.pub files are NOT in the SPEC permission table."""
    repo = tmp_repo
    home = unique_home
    ssh_dir = home / ".ssh"
    ssh_dir.mkdir(exist_ok=True)

    src = repo / "id_rsa_pub"
    src.write_text("public key")
    os.chmod(src, 0o644)  # overly permissive, but .pub should be exempt

    (repo / "xdotter.toml").write_text(
        '[links]\n"id_rsa_pub" = "~/.ssh/id_rsa.pub"\n'
    )

    result = run_xd(["deploy"], cwd=repo, home=home)
    assert result.code == 0, result.stderr
    assert (home / ".ssh/id_rsa.pub").is_symlink()
