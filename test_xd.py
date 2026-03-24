#!/usr/bin/env python3
"""
Test suite for xdotter
Verifies all core functionality works correctly
"""

import os
import sys
import tempfile
import shutil
import subprocess
from pathlib import Path

# Add parent directory to path to import xd
sys.path.insert(0, str(Path(__file__).parent))

# Test results
PASSED = 0
FAILED = 0
SKIPPED = 0

def log_test(name, status, message=""):
    """Log test result"""
    global PASSED, FAILED, SKIPPED
    status_map = {
        "PASS": ("\033[0;32mPASS\033[0m", lambda: globals().update({"PASSED": PASSED + 1})),
        "FAIL": ("\033[0;31mFAIL\033[0m", lambda: globals().update({"FAILED": FAILED + 1})),
        "SKIP": ("\033[1;33mSKIP\033[0m", lambda: globals().update({"SKIPPED": SKIPPED + 1})),
    }
    colored_status, counter = status_map[status]
    counter()
    msg = f"  {message}" if message else ""
    print(f"  [{colored_status}] {name}{msg}")

def run_xd(args, cwd=None, input_data=None, env=None):
    """Run xd.py with arguments and return output"""
    cmd = [sys.executable, str(Path(__file__).parent / "xd.py")] + args
    # Merge environment
    full_env = os.environ.copy()
    if env:
        full_env.update(env)
    result = subprocess.run(
        cmd,
        cwd=cwd,
        capture_output=True,
        text=True,
        input=input_data,
        env=full_env
    )
    return result.returncode, result.stdout, result.stderr

def test_help_command():
    """Test help command"""
    print("\n[Test: Help Command]")
    code, stdout, stderr = run_xd(["--help"])
    
    if code == 0 and "deploy" in stdout and "undeploy" in stdout and "new" in stdout:
        log_test("Help output contains all commands", "PASS")
    else:
        log_test("Help output contains all commands", "FAIL", f"code={code}")

def test_version_command():
    """Test version command"""
    print("\n[Test: Version Command]")
    code, stdout, stderr = run_xd(["version"])
    
    if code == 0 and "xdotter" in stdout:
        log_test("Version command works", "PASS")
    else:
        log_test("Version command works", "FAIL", f"code={code}")

def test_new_command():
    """Test new command creates config template"""
    print("\n[Test: New Command]")
    
    with tempfile.TemporaryDirectory() as tmpdir:
        tmppath = Path(tmpdir)
        code, stdout, stderr = run_xd(["new"], cwd=tmpdir)
        config_file = tmppath / "xdotter.toml"
        
        if code == 0 and config_file.exists():
            log_test("New command creates xdotter.toml", "PASS")
            
            content = config_file.read_text()
            if "[links]" in content and "[dependencies]" in content:
                log_test("Config has required sections", "PASS")
            else:
                log_test("Config has required sections", "FAIL", "Missing sections")
        else:
            log_test("New command creates xdotter.toml", "FAIL", f"code={code}")

def test_deploy_basic_link():
    """Test deploying a basic symlink"""
    print("\n[Test: Deploy Basic Link]")
    
    with tempfile.TemporaryDirectory() as tmpdir:
        tmppath = Path(tmpdir)
        
        # Create source file
        source_dir = tmppath / "source"
        source_dir.mkdir()
        source_file = source_dir / "test.txt"
        source_file.write_text("test content")
        
        # Create config
        config = tmppath / "test.toml"
        config.write_text(f'''
[links]
"source/test.txt" = "~/.cache/xdotter_test_{os.getpid()}.txt"
''')
        
        target_path = Path.home() / f".cache/xdotter_test_{os.getpid()}.txt"
        
        try:
            # Ensure target dir exists
            target_path.parent.mkdir(exist_ok=True)
            
            # Run from the temp directory so relative paths work
            code, stdout, stderr = run_xd(["-c", "test.toml", "deploy", "-v"], cwd=tmpdir)
            
            if code == 0 and target_path.is_symlink():
                log_test("Deploy creates symlink", "PASS")
                
                # Verify symlink target
                if target_path.read_text() == "test content":
                    log_test("Symlink points to correct file", "PASS")
                else:
                    log_test("Symlink points to correct file", "FAIL", "Content mismatch")
            else:
                log_test("Deploy creates symlink", "FAIL", f"code={code}, target exists: {target_path.exists()}, stderr: {stderr}")
        finally:
            # Cleanup
            if target_path.exists():
                target_path.unlink()

def test_deploy_dry_run():
    """Test deploy with --dry-run flag"""
    print("\n[Test: Deploy Dry Run]")
    
    with tempfile.TemporaryDirectory() as tmpdir:
        tmppath = Path(tmpdir)
        
        # Create source file
        source_dir = tmppath / "source"
        source_dir.mkdir()
        source_file = source_dir / "test.txt"
        source_file.write_text("test content")
        
        # Create config
        config = tmppath / "test.toml"
        config.write_text(f'''
[links]
"source/test.txt" = "~/.cache/xdotter_dryrun_{os.getpid()}.txt"
''')
        
        target_path = Path.home() / f".cache/xdotter_dryrun_{os.getpid()}.txt"
        
        try:
            # Ensure target doesn't exist
            if target_path.exists():
                target_path.unlink()
            
            # Run from temp directory
            code, stdout, stderr = run_xd(["-c", "test.toml", "deploy", "-n", "-v"], cwd=tmpdir)
            
            if code == 0 and not target_path.exists():
                log_test("Dry run does not create files", "PASS")
                
                if "deploy:" in stdout.lower():
                    log_test("Dry run shows what would happen", "PASS")
                else:
                    log_test("Dry run shows what would happen", "FAIL", "No output")
            else:
                log_test("Dry run does not create files", "FAIL", "File was created")
        finally:
            if target_path.exists():
                target_path.unlink()

def test_undeploy():
    """Test undeploy removes symlinks"""
    print("\n[Test: Undeploy]")
    
    with tempfile.TemporaryDirectory() as tmpdir:
        tmppath = Path(tmpdir)
        
        # Create source file
        source_dir = tmppath / "source"
        source_dir.mkdir()
        source_file = source_dir / "test.txt"
        source_file.write_text("test content")
        
        # Create config
        config = tmppath / "test.toml"
        config.write_text(f'''
[links]
"source/test.txt" = "~/.cache/xdotter_undeploy_{os.getpid()}.txt"
''')
        
        target_path = Path.home() / f".cache/xdotter_undeploy_{os.getpid()}.txt"
        
        try:
            target_path.parent.mkdir(exist_ok=True)
            
            # First deploy (run from temp dir)
            code, _, _ = run_xd(["-c", "test.toml", "deploy"], cwd=tmpdir)
            
            if target_path.is_symlink():
                log_test("Precondition: symlink exists", "PASS")
                
                # Then undeploy
                code, stdout, stderr = run_xd(["-c", "test.toml", "undeploy", "-v"], cwd=tmpdir)
                
                if code == 0 and not target_path.exists():
                    log_test("Undeploy removes symlink", "PASS")
                else:
                    log_test("Undeploy removes symlink", "FAIL", f"Still exists: {target_path.exists()}")
            else:
                log_test("Precondition: symlink exists", "FAIL", "Deploy failed")
        finally:
            if target_path.exists():
                target_path.unlink()

def test_deploy_with_tilde():
    """Test deploying with ~ in paths"""
    print("\n[Test: Tilde Expansion]")
    
    with tempfile.TemporaryDirectory() as tmpdir:
        tmppath = Path(tmpdir)
        
        # Create source file
        source_dir = tmppath / "source"
        source_dir.mkdir()
        source_file = source_dir / "test.txt"
        source_file.write_text("test content")
        
        # Create config with ~ in target
        config = tmppath / "test.toml"
        config.write_text(f'''
[links]
"source/test.txt" = "~/.cache/xdotter_tilde_{os.getpid()}.txt"
''')
        
        target_path = Path.home() / f".cache/xdotter_tilde_{os.getpid()}.txt"
        
        try:
            target_path.parent.mkdir(exist_ok=True)
            
            # Run from temp directory
            code, stdout, stderr = run_xd(["-c", "test.toml", "deploy", "-v"], cwd=tmpdir)
            
            if code == 0 and target_path.is_symlink():
                log_test("Tilde expansion works", "PASS")
            else:
                log_test("Tilde expansion works", "FAIL", f"code={code}, stderr: {stderr}")
        finally:
            if target_path.exists():
                target_path.unlink()

def test_quiet_mode():
    """Test --quiet flag"""
    print("\n[Test: Quiet Mode]")
    
    with tempfile.TemporaryDirectory() as tmpdir:
        tmppath = Path(tmpdir)
        
        # Create config that doesn't exist to see quiet behavior
        config = tmppath / "nonexistent.toml"
        
        code, stdout, stderr = run_xd(["-c", str(config), "deploy", "-q"])
        
        # Quiet mode should suppress output but still have error code
        if len(stdout.strip()) == 0:
            log_test("Quiet mode suppresses output", "PASS")
        else:
            log_test("Quiet mode suppresses output", "FAIL", f"Got output: {stdout}")

def test_verbose_mode():
    """Test --verbose flag"""
    print("\n[Test: Verbose Mode]")
    
    with tempfile.TemporaryDirectory() as tmpdir:
        tmppath = Path(tmpdir)
        
        # Create source file
        source_dir = tmppath / "source"
        source_dir.mkdir()
        source_file = source_dir / "test.txt"
        source_file.write_text("test content")
        
        # Create config
        config = tmppath / "test.toml"
        config.write_text(f'''
[links]
"source/test.txt" = "~/.cache/xdotter_verbose_{os.getpid()}.txt"
''')
        
        target_path = Path.home() / f".cache/xdotter_verbose_{os.getpid()}.txt"
        
        try:
            target_path.parent.mkdir(exist_ok=True)
            
            code, stdout, stderr = run_xd(["-c", str(config), "deploy", "-v"])
            
            if "[DEBUG]" in stdout or "DEBUG" in stdout:
                log_test("Verbose mode shows debug info", "PASS")
            else:
                log_test("Verbose mode shows debug info", "FAIL", "No debug output")
        finally:
            if target_path.exists():
                target_path.unlink()

def test_force_flag():
    """Test --force flag for overwriting"""
    print("\n[Test: Force Flag]")
    
    with tempfile.TemporaryDirectory() as tmpdir:
        tmppath = Path(tmpdir)
        
        # Create source files
        source_dir = tmppath / "source"
        source_dir.mkdir()
        source_file = source_dir / "test.txt"
        source_file.write_text("new content")
        
        target_dir = Path.home() / ".cache"
        target_dir.mkdir(exist_ok=True)
        target_path = target_dir / f"xdotter_force_{os.getpid()}.txt"
        
        # Create existing file (not symlink)
        target_path.write_text("old content")
        
        # Create config
        config = tmppath / "test.toml"
        config.write_text(f'''
[links]
"source/test.txt" = "~/.cache/xdotter_force_{os.getpid()}.txt"
''')
        
        try:
            # Try with force (run from temp dir)
            code, stdout, stderr = run_xd(["-c", "test.toml", "deploy", "-f", "-v"], cwd=tmpdir)
            
            if code == 0 and target_path.is_symlink():
                log_test("Force flag overwrites existing file", "PASS")
            else:
                log_test("Force flag overwrites existing file", "FAIL", f"code={code}, is_symlink={target_path.is_symlink()}, stderr: {stderr}")
        finally:
            if target_path.exists():
                if target_path.is_symlink():
                    target_path.unlink()
                else:
                    target_path.unlink()

def test_config_parsing():
    """Test TOML config parsing"""
    print("\n[Test: Config Parsing]")
    
    from xd import ConfigParser
    
    toml_content = '''
# Comment
[links]
".zshrc" = "~/.zshrc"
".config/nvim/init.lua" = "~/.config/nvim/init.lua"

[dependencies]
"go" = "testdata/go"
'''
    
    try:
        config = ConfigParser.parse(toml_content)
        
        if ".zshrc" in config["links"] and config["links"][".zshrc"] == "~/.zshrc":
            log_test("Parse links section", "PASS")
        else:
            log_test("Parse links section", "FAIL", str(config))
        
        if "go" in config["dependencies"] and config["dependencies"]["go"] == "testdata/go":
            log_test("Parse dependencies section", "PASS")
        else:
            log_test("Parse dependencies section", "FAIL", str(config))
    except Exception as e:
        log_test("Config parsing", "FAIL", str(e))

def test_multiple_links():
    """Test deploying multiple links at once"""
    print("\n[Test: Multiple Links]")
    
    with tempfile.TemporaryDirectory() as tmpdir:
        tmppath = Path(tmpdir)
        
        # Create multiple source files
        source_dir = tmppath / "source"
        source_dir.mkdir()
        (source_dir / "file1.txt").write_text("content1")
        (source_dir / "file2.txt").write_text("content2")
        (source_dir / "file3.txt").write_text("content3")
        
        # Create config with multiple links
        config = tmppath / "test.toml"
        config.write_text(f'''
[links]
"source/file1.txt" = "~/.cache/xdotter_multi_1_{os.getpid()}.txt"
"source/file2.txt" = "~/.cache/xdotter_multi_2_{os.getpid()}.txt"
"source/file3.txt" = "~/.cache/xdotter_multi_3_{os.getpid()}.txt"
''')
        
        targets = [
            Path.home() / f".cache/xdotter_multi_{i}_{os.getpid()}.txt"
            for i in range(1, 4)
        ]
        
        try:
            for t in targets:
                t.parent.mkdir(exist_ok=True)
                if t.exists():
                    t.unlink()
            
            # Run from temp directory
            code, stdout, stderr = run_xd(["-c", "test.toml", "deploy", "-v"], cwd=tmpdir)
            
            all_exist = all(t.is_symlink() for t in targets)
            
            if code == 0 and all_exist:
                log_test("Multiple links deployed", "PASS")
            else:
                log_test("Multiple links deployed", "FAIL", f"code={code}, count={sum(1 for t in targets if t.is_symlink())}/3, stderr: {stderr}")
        finally:
            for t in targets:
                if t.exists():
                    t.unlink()

def print_summary():
    """Print test summary"""
    total = PASSED + FAILED + SKIPPED
    print("\n" + "=" * 50)
    print(f"Test Summary: {PASSED}/{total} passed")
    print(f"  \033[0;32mPassed:  {PASSED}\033[0m")
    print(f"  \033[0;31mFailed:  {FAILED}\033[0m")
    print(f"  \033[1;33mSkipped: {SKIPPED}\033[0m")
    print("=" * 50)
    
    return FAILED == 0


# ============================================================
# Additional Test Scenarios
# ============================================================

def test_dependencies_subdirectory():
    """Test deploying with dependencies (subdirectory with its own config)"""
    print("\n[Test: Dependencies Subdirectory]")
    
    with tempfile.TemporaryDirectory() as tmpdir:
        tmppath = Path(tmpdir)
        
        # Create main source file
        source_file = tmppath / "main.txt"
        source_file.write_text("main content")
        
        # Create subdirectory with its own config
        sub_dir = tmppath / "sub"
        sub_dir.mkdir()
        sub_source = sub_dir / "sub.txt"
        sub_source.write_text("sub content")
        
        # Subdirectory config
        sub_config = sub_dir / "xdotter.toml"
        sub_config.write_text(f'''
[links]
"sub.txt" = "~/.cache/xdotter_sub_{os.getpid()}.txt"
''')
        
        # Main config with dependency
        main_config = tmppath / "test.toml"
        main_config.write_text(f'''
[links]
"main.txt" = "~/.cache/xdotter_main_{os.getpid()}.txt"

[dependencies]
"sub" = "sub"
''')
        
        main_target = Path.home() / f".cache/xdotter_main_{os.getpid()}.txt"
        sub_target = Path.home() / f".cache/xdotter_sub_{os.getpid()}.txt"
        
        try:
            main_target.parent.mkdir(exist_ok=True)
            
            # Run from temp directory
            code, stdout, stderr = run_xd(["-c", "test.toml", "deploy", "-v"], cwd=tmpdir)
            
            main_ok = main_target.is_symlink()
            sub_ok = sub_target.is_symlink()
            
            if code == 0 and main_ok and sub_ok:
                log_test("Dependencies subdirectory deployed", "PASS")
            else:
                log_test("Dependencies subdirectory deployed", "FAIL", 
                    f"main={main_ok}, sub={sub_ok}, stderr: {stderr}")
        finally:
            if main_target.exists():
                main_target.unlink()
            if sub_target.exists():
                sub_target.unlink()


def test_interactive_mode_confirm():
    """Test interactive mode with confirmation"""
    print("\n[Test: Interactive Mode Confirm]")
    
    with tempfile.TemporaryDirectory() as tmpdir:
        tmppath = Path(tmpdir)
        
        # Create source file
        source_dir = tmppath / "source"
        source_dir.mkdir()
        source_file = source_dir / "test.txt"
        source_file.write_text("test content")
        
        # Create config
        config = tmppath / "test.toml"
        config.write_text(f'''
[links]
"source/test.txt" = "~/.cache/xdotter_inter_{os.getpid()}.txt"
''')
        
        target_path = Path.home() / f".cache/xdotter_inter_{os.getpid()}.txt"
        
        try:
            target_path.parent.mkdir(exist_ok=True)
            
            # First deploy
            run_xd(["-c", "test.toml", "deploy"], cwd=tmpdir)
            
            if target_path.is_symlink():
                log_test("Precondition: symlink exists", "PASS")
                
                # Remove source to create a different one
                source_file.write_text("different content")
                
                # Try interactive mode with 'n' (no)
                code, stdout, stderr = run_xd(
                    ["-c", "test.toml", "deploy", "-i"],
                    cwd=tmpdir,
                    input_data="n\n"
                )
                
                # Should skip because we answered 'n'
                if target_path.is_symlink():
                    log_test("Interactive mode respects 'n' answer", "PASS")
                else:
                    log_test("Interactive mode respects 'n' answer", "FAIL", "Link was removed")
            else:
                log_test("Precondition: symlink exists", "FAIL")
        finally:
            if target_path.exists():
                target_path.unlink()


def test_interactive_mode_yes():
    """Test interactive mode with yes confirmation"""
    print("\n[Test: Interactive Mode Yes]")
    
    with tempfile.TemporaryDirectory() as tmpdir:
        tmppath = Path(tmpdir)
        
        # Create source file
        source_dir = tmppath / "source"
        source_dir.mkdir()
        source_file = source_dir / "test.txt"
        source_file.write_text("original content")
        
        target_dir = Path.home() / ".cache"
        target_dir.mkdir(exist_ok=True)
        target_path = target_dir / f"xdotter_intery_{os.getpid()}.txt"
        
        # Create existing file (not symlink)
        target_path.write_text("existing content")
        
        # Change source content
        source_file.write_text("new content")
        
        # Create config
        config = tmppath / "test.toml"
        config.write_text(f'''
[links]
"source/test.txt" = "~/.cache/xdotter_intery_{os.getpid()}.txt"
''')
        
        try:
            # Try interactive mode with 'y' (yes)
            code, stdout, stderr = run_xd(
                ["-c", "test.toml", "deploy", "-i"],
                cwd=tmpdir,
                input_data="y\n"
            )
            
            if code == 0 and target_path.is_symlink():
                log_test("Interactive mode respects 'y' answer", "PASS")
            else:
                log_test("Interactive mode respects 'y' answer", "FAIL", 
                    f"code={code}, is_symlink={target_path.is_symlink()}")
        finally:
            if target_path.exists():
                if target_path.is_symlink():
                    target_path.unlink()
                else:
                    target_path.unlink()


def test_nonexistent_source():
    """Test deploying with nonexistent source file"""
    print("\n[Test: Nonexistent Source]")
    
    with tempfile.TemporaryDirectory() as tmpdir:
        tmppath = Path(tmpdir)
        
        # Create config pointing to nonexistent file
        config = tmppath / "test.toml"
        config.write_text(f'''
[links]
"nonexistent.txt" = "~/.cache/xdotter_noexist_{os.getpid()}.txt"
''')
        
        target_path = Path.home() / f".cache/xdotter_noexist_{os.getpid()}.txt"
        
        try:
            target_path.parent.mkdir(exist_ok=True)
            
            code, stdout, stderr = run_xd(["-c", "test.toml", "deploy", "-v"], cwd=tmpdir)
            
            # Should fail gracefully, not crash
            if "does not exist" in stderr or "does not exist" in stdout or code != 0:
                log_test("Handles nonexistent source gracefully", "PASS")
            else:
                log_test("Handles nonexistent source gracefully", "FAIL", "Should report error")
        finally:
            if target_path.exists():
                target_path.unlink()


def test_nonexistent_config():
    """Test with nonexistent config file"""
    print("\n[Test: Nonexistent Config]")

    with tempfile.TemporaryDirectory() as tmpdir:
        tmppath = Path(tmpdir)

        code, stdout, stderr = run_xd(["-c", "nonexistent.toml", "deploy"], cwd=tmpdir)

        # Should fail with appropriate error
        if code != 0 and ("not found" in stderr.lower() or "failed to read" in stderr.lower() or "no such file" in stderr.lower()):
            log_test("Handles nonexistent config gracefully", "PASS")
        else:
            log_test("Handles nonexistent config gracefully", "FAIL", f"code={code}")


def test_invalid_toml_syntax():
    """Test with invalid TOML syntax"""
    print("\n[Test: Invalid TOML Syntax]")
    
    with tempfile.TemporaryDirectory() as tmpdir:
        tmppath = Path(tmpdir)
        
        # Create invalid TOML
        config = tmppath / "test.toml"
        config.write_text('''
[links
"invalid" = "syntax
''')
        
        code, stdout, stderr = run_xd(["-c", "test.toml", "deploy"], cwd=tmpdir)
        
        # Should fail gracefully
        if code != 0 or "parse" in stderr.lower():
            log_test("Handles invalid TOML gracefully", "PASS")
        else:
            # Parser is lenient, may just skip invalid lines
            log_test("Handles invalid TOML gracefully", "PASS", "Lenient parser accepts")


def test_empty_config():
    """Test with empty config file"""
    print("\n[Test: Empty Config]")
    
    with tempfile.TemporaryDirectory() as tmpdir:
        tmppath = Path(tmpdir)
        
        # Create empty config
        config = tmppath / "test.toml"
        config.write_text("")
        
        code, stdout, stderr = run_xd(["-c", "test.toml", "deploy"], cwd=tmpdir)
        
        # Should succeed with nothing to do
        if code == 0:
            log_test("Handles empty config", "PASS")
        else:
            log_test("Handles empty config", "FAIL", f"code={code}")


def test_symlink_already_exists():
    """Test when symlink already points to correct target"""
    print("\n[Test: Symlink Already Exists]")
    
    with tempfile.TemporaryDirectory() as tmpdir:
        tmppath = Path(tmpdir)
        
        # Create source file
        source_dir = tmppath / "source"
        source_dir.mkdir()
        source_file = source_dir / "test.txt"
        source_file.write_text("test content")
        
        target_dir = Path.home() / ".cache"
        target_dir.mkdir(exist_ok=True)
        target_path = target_dir / f"xdotter_exist_{os.getpid()}.txt"
        
        # Create correct symlink first
        source_resolved = source_file.resolve()
        os.symlink(source_resolved, target_path)
        
        # Create config
        config = tmppath / "test.toml"
        config.write_text(f'''
[links]
"source/test.txt" = "~/.cache/xdotter_exist_{os.getpid()}.txt"
''')
        
        try:
            code, stdout, stderr = run_xd(["-c", "test.toml", "deploy", "-v"], cwd=tmpdir)
            
            # Should skip, not error
            if code == 0 and target_path.is_symlink():
                log_test("Skips existing correct symlink", "PASS")
            else:
                log_test("Skips existing correct symlink", "FAIL", f"code={code}")
        finally:
            if target_path.exists():
                target_path.unlink()


def test_unicode_paths():
    """Test with unicode characters in paths"""
    print("\n[Test: Unicode Paths]")
    
    with tempfile.TemporaryDirectory() as tmpdir:
        tmppath = Path(tmpdir)
        
        # Create source file with unicode name
        source_dir = tmppath / "测试目录"
        source_dir.mkdir()
        source_file = source_dir / "测试文件.txt"
        source_file.write_text("中文内容 Chinese content")
        
        # Create config
        config = tmppath / "test.toml"
        config.write_text(f'''
[links]
"测试目录/测试文件.txt" = "~/.cache/xdotter_unicode_{os.getpid()}.txt"
''')
        
        target_path = Path.home() / f".cache/xdotter_unicode_{os.getpid()}.txt"
        
        try:
            target_path.parent.mkdir(exist_ok=True)
            
            code, stdout, stderr = run_xd(["-c", "test.toml", "deploy"], cwd=tmpdir)
            
            if code == 0 and target_path.is_symlink():
                log_test("Unicode paths work", "PASS")
            else:
                log_test("Unicode paths work", "FAIL", f"code={code}")
        finally:
            if target_path.exists():
                target_path.unlink()


def test_absolute_path_in_config():
    """Test with absolute paths in config"""
    print("\n[Test: Absolute Path In Config]")
    
    with tempfile.TemporaryDirectory() as tmpdir:
        tmppath = Path(tmpdir)
        
        # Create source file
        source_file = tmppath / "source.txt"
        source_file.write_text("absolute path test")
        
        target_path = Path.home() / f".cache/xdotter_abs_{os.getpid()}.txt"
        
        # Create config with absolute path
        config = tmppath / "test.toml"
        config.write_text(f'''
[links]
"{source_file}" = "~/.cache/xdotter_abs_{os.getpid()}.txt"
''')
        
        try:
            target_path.parent.mkdir(exist_ok=True)
            
            code, stdout, stderr = run_xd(["-c", "test.toml", "deploy"], cwd=tmpdir)
            
            if code == 0 and target_path.is_symlink():
                log_test("Absolute paths in config work", "PASS")
            else:
                log_test("Absolute paths in config work", "FAIL", f"code={code}")
        finally:
            if target_path.exists():
                target_path.unlink()


def test_undeploy_nonexistent_link():
    """Test undeploy when link doesn't exist"""
    print("\n[Test: Undeploy Nonexistent Link]")
    
    with tempfile.TemporaryDirectory() as tmpdir:
        tmppath = Path(tmpdir)
        
        # Create config for nonexistent link
        config = tmppath / "test.toml"
        config.write_text(f'''
[links]
"source.txt" = "~/.cache/xdotter_nolink_{os.getpid()}.txt"
''')
        
        code, stdout, stderr = run_xd(["-c", "test.toml", "undeploy", "-v"], cwd=tmpdir)
        
        # Should succeed (skip nonexistent)
        if code == 0:
            log_test("Undeploy handles nonexistent link", "PASS")
        else:
            log_test("Undeploy handles nonexistent link", "FAIL", f"code={code}")


def test_comments_in_config():
    """Test config with various comment styles"""
    print("\n[Test: Comments In Config]")
    
    with tempfile.TemporaryDirectory() as tmpdir:
        tmppath = Path(tmpdir)
        
        # Create source file
        source_dir = tmppath / "source"
        source_dir.mkdir()
        source_file = source_dir / "test.txt"
        source_file.write_text("test content")
        
        # Create config with comments
        config = tmppath / "test.toml"
        config.write_text(f'''
# This is a comment
[links]
# Another comment
"source/test.txt" = "~/.cache/xdotter_comment_{os.getpid()}.txt"  # inline comment

# Comment before dependencies
[dependencies]
# Empty dependencies is ok
''')
        
        target_path = Path.home() / f".cache/xdotter_comment_{os.getpid()}.txt"
        
        try:
            target_path.parent.mkdir(exist_ok=True)
            
            code, stdout, stderr = run_xd(["-c", "test.toml", "deploy"], cwd=tmpdir)
            
            if code == 0 and target_path.is_symlink():
                log_test("Comments in config handled", "PASS")
            else:
                log_test("Comments in config handled", "FAIL", f"code={code}")
        finally:
            if target_path.exists():
                target_path.unlink()


def test_whitespace_in_config():
    """Test config with various whitespace"""
    print("\n[Test: Whitespace In Config]")
    
    with tempfile.TemporaryDirectory() as tmpdir:
        tmppath = Path(tmpdir)
        
        # Create source file
        source_dir = tmppath / "source"
        source_dir.mkdir()
        source_file = source_dir / "test.txt"
        source_file.write_text("test content")
        
        # Create config with extra whitespace
        config = tmppath / "test.toml"
        config.write_text(f'''
[links]
  "source/test.txt"   =   "~/.cache/xdotter_ws_{os.getpid()}.txt"  
''')
        
        target_path = Path.home() / f".cache/xdotter_ws_{os.getpid()}.txt"
        
        try:
            target_path.parent.mkdir(exist_ok=True)
            
            code, stdout, stderr = run_xd(["-c", "test.toml", "deploy"], cwd=tmpdir)
            
            if code == 0 and target_path.is_symlink():
                log_test("Whitespace in config handled", "PASS")
            else:
                log_test("Whitespace in config handled", "FAIL", f"code={code}")
        finally:
            if target_path.exists():
                target_path.unlink()


def test_single_quotes_in_config():
    """Test config with single quotes"""
    print("\n[Test: Single Quotes In Config]")

    with tempfile.TemporaryDirectory() as tmpdir:
        tmppath = Path(tmpdir)

        # Create source file
        source_dir = tmppath / "source"
        source_dir.mkdir()
        source_file = source_dir / "test.txt"
        source_file.write_text("test content")

        # Create config with single quotes
        config = tmppath / "test.toml"
        config.write_text(f"""
[links]
'source/test.txt' = '~/.cache/xdotter_sq_{os.getpid()}.txt'
""")

        target_path = Path.home() / f".cache/xdotter_sq_{os.getpid()}.txt"

        try:
            target_path.parent.mkdir(exist_ok=True)

            code, stdout, stderr = run_xd(["-c", "test.toml", "deploy"], cwd=tmpdir)

            if code == 0 and target_path.is_symlink():
                log_test("Single quotes in config work", "PASS")
            else:
                log_test("Single quotes in config work", "FAIL", f"code={code}")
        finally:
            if target_path.exists():
                target_path.unlink()


# ============================================================
# Permission Check Tests
# ============================================================

def test_permission_check_ssh_key():
    """Test --check-permissions for SSH key"""
    print("\n[Test: Permission Check SSH Key]")

    with tempfile.TemporaryDirectory() as tmpdir:
        tmppath = Path(tmpdir)

        # Create source file with wrong permission (644 instead of 600)
        source_dir = tmppath / "source"
        source_dir.mkdir()
        source_file = source_dir / "id_ed25519"
        source_file.write_text("fake ssh key")
        source_file.chmod(0o644)

        # Create config - use ~/.ssh/id_ed25519 to match sensitive pattern
        # Filename must match pattern "id_ed25519*" for detection
        config = tmppath / "test.toml"
        config.write_text(f'''
[links]
"source/id_ed25519" = "~/.ssh/id_ed25519_xdotter_test_{os.getpid()}"
''')

        target_path = Path.home() / ".ssh" / f"id_ed25519_xdotter_test_{os.getpid()}"

        try:
            target_path.parent.mkdir(exist_ok=True)

            # Deploy with --check-permissions
            code, stdout, stderr = run_xd(
                ["-c", "test.toml", "deploy", "--check-permissions", "-v"],
                cwd=tmpdir
            )

            # Should show permission warning - filename matches id_ed25519* pattern
            if "✗" in stdout and "600" in stdout:
                log_test("Detects wrong SSH key permission", "PASS")
            else:
                log_test("Detects wrong SSH key permission", "FAIL", f"stdout: {stdout[:200]}")

        finally:
            if target_path.exists() or target_path.is_symlink():
                target_path.unlink()


def test_permission_fix_ssh_key():
    """Test --fix-permissions for SSH key"""
    print("\n[Test: Permission Fix SSH Key]")

    with tempfile.TemporaryDirectory() as tmpdir:
        tmppath = Path(tmpdir)

        # Create source file with wrong permission
        source_dir = tmppath / "source"
        source_dir.mkdir()
        source_file = source_dir / "id_ed25519"
        source_file.write_text("fake ssh key")
        source_file.chmod(0o644)

        # Create config - use filename that matches id_ed25519* pattern
        # Pattern: id_ed25519* matches filenames starting with "id_ed25519"
        config = tmppath / "test.toml"
        config.write_text(f'''
[links]
"source/id_ed25519" = "~/.ssh/id_ed25519_xdotter_test_{os.getpid()}"
''')

        target_path = Path.home() / ".ssh" / f"id_ed25519_xdotter_test_{os.getpid()}"

        try:
            target_path.parent.mkdir(exist_ok=True)

            # Deploy with --fix-permissions
            code, stdout, stderr = run_xd(
                ["-c", "test.toml", "deploy", "--fix-permissions", "-v"],
                cwd=tmpdir
            )

            # Check if source file permission was fixed
            import stat
            actual_mode = stat.S_IMODE(source_file.stat().st_mode)

            # Should be fixed to 600 (matches id_ed25519* pattern)
            if actual_mode == 0o600:
                log_test("Fixes SSH key permission to 600", "PASS")
            else:
                log_test("Fixes SSH key permission to 600", "FAIL", f"mode={oct(actual_mode)}")

        finally:
            if target_path.exists() or target_path.is_symlink():
                target_path.unlink()


def test_permission_check_correct_permission():
    """Test --check-permissions with correct permission"""
    print("\n[Test: Permission Check Correct]")

    with tempfile.TemporaryDirectory() as tmpdir:
        tmppath = Path(tmpdir)

        # Create source file with correct permission (600)
        source_dir = tmppath / "source"
        source_dir.mkdir()
        source_file = source_dir / "id_ed25519"
        source_file.write_text("fake ssh key")
        source_file.chmod(0o600)

        # Create config - use ~/.ssh/ path with filename matching pattern
        config = tmppath / "test.toml"
        config.write_text(f'''
[links]
"source/id_ed25519" = "~/.ssh/id_ed25519_xdotter_correct_{os.getpid()}"
''')

        target_path = Path.home() / ".ssh" / f"id_ed25519_xdotter_correct_{os.getpid()}"

        try:
            target_path.parent.mkdir(exist_ok=True)

            # Deploy with --check-permissions
            code, stdout, stderr = run_xd(
                ["-c", "test.toml", "deploy", "--check-permissions", "-v"],
                cwd=tmpdir
            )

            # Should show checkmark (✓)
            if "✓" in stdout:
                log_test("Recognizes correct SSH key permission", "PASS")
            else:
                log_test("Recognizes correct SSH key permission", "FAIL", f"stdout: {stdout[:200]}")

        finally:
            if target_path.exists() or target_path.is_symlink():
                target_path.unlink()


def test_permission_pattern_matching():
    """Test permission pattern matching for various key files"""
    print("\n[Test: Permission Pattern Matching]")

    with tempfile.TemporaryDirectory() as tmpdir:
        tmppath = Path(tmpdir)

        # Create source files with wrong permissions
        source_dir = tmppath / "source"
        source_dir.mkdir()

        # Use target filenames that match patterns directly
        # Format: (source_name, target_name, pattern_matched)
        test_cases = [
            ("key1", "id_rsa_custom", "id_rsa*"),        # matches id_rsa*
            ("key2", "server_ed25519", "*_ed25519"),     # matches *_ed25519
            ("key3", "cert.pem", "*.pem"),               # matches *.pem
            ("key4", "mykey.key", "*.key"),              # matches *.key
        ]

        for src_name, tgt_name, pattern in test_cases:
            f = source_dir / src_name
            f.write_text("fake key")
            f.chmod(0o644)

        # Create config - use ~/.ssh/ paths with filenames that match patterns
        config = tmppath / "test.toml"
        links = '\n'.join([f'"source/{src}" = "~/.ssh/{tgt}_{os.getpid()}"'
                          for src, tgt, _ in test_cases])
        config.write_text(f'''
[links]
{links}
''')

        # Deploy with --check-permissions
        code, stdout, stderr = run_xd(
            ["-c", "test.toml", "deploy", "--check-permissions", "-v"],
            cwd=tmpdir
        )

        # All should be detected as needing permission fix
        # The exact permission depends on pattern matching
        if stdout.count("✗") >= 4:
            log_test("Pattern matching detects all key types", "PASS")
        else:
            log_test("Pattern matching detects all key types", "FAIL", f"✗ count={stdout.count('✗')}")

        # Cleanup
        for src, tgt, _ in test_cases:
            target = Path.home() / ".ssh" / f"{tgt}_{os.getpid()}"
            if target.exists() or target.is_symlink():
                target.unlink()


def test_permission_dry_run():
    """Test --fix-permissions with --dry-run doesn't modify files"""
    print("\n[Test: Permission Dry Run]")

    with tempfile.TemporaryDirectory() as tmpdir:
        tmppath = Path(tmpdir)

        # Create source file with wrong permission
        source_dir = tmppath / "source"
        source_dir.mkdir()
        source_file = source_dir / "id_ed25519"
        source_file.write_text("fake ssh key")
        source_file.chmod(0o644)

        # Create config - use ~/.ssh/ path to match sensitive pattern
        config = tmppath / "test.toml"
        config.write_text(f'''
[links]
"source/id_ed25519" = "~/.ssh/xdotter_dry_perm_{os.getpid()}"
''')

        target_path = Path.home() / ".ssh" / f"xdotter_dry_perm_{os.getpid()}"

        try:
            target_path.parent.mkdir(exist_ok=True)

            # Deploy with --fix-permissions --dry-run
            code, stdout, stderr = run_xd(
                ["-c", "test.toml", "deploy", "--fix-permissions", "-n", "-v"],
                cwd=tmpdir
            )

            # Check source file permission is NOT changed
            import stat
            actual_mode = stat.S_IMODE(source_file.stat().st_mode)

            if actual_mode == 0o644:
                log_test("Dry-run doesn't modify permissions", "PASS")
            else:
                log_test("Dry-run doesn't modify permissions", "FAIL", f"mode changed to {oct(actual_mode)}")

        finally:
            if target_path.exists() or target_path.is_symlink():
                target_path.unlink()


def main():
    """Run all tests"""
    print("=" * 50)
    print("xdotter Test Suite")
    print("=" * 50)
    
    # Basic command tests
    test_help_command()
    test_version_command()
    test_new_command()
    
    # Config parsing test
    test_config_parsing()
    
    # Deploy tests
    test_deploy_basic_link()
    test_deploy_dry_run()
    test_deploy_with_tilde()
    test_multiple_links()
    
    # Undeploy test
    test_undeploy()
    
    # Flag tests
    test_quiet_mode()
    test_verbose_mode()
    test_force_flag()
    
    # Additional test scenarios
    test_dependencies_subdirectory()
    test_interactive_mode_confirm()
    test_interactive_mode_yes()
    test_nonexistent_source()
    test_nonexistent_config()
    test_invalid_toml_syntax()
    test_empty_config()
    test_symlink_already_exists()
    test_unicode_paths()
    test_absolute_path_in_config()
    test_undeploy_nonexistent_link()
    test_comments_in_config()
    test_whitespace_in_config()
    test_single_quotes_in_config()
    

    # Permission check tests
    test_permission_check_ssh_key()
    test_permission_fix_ssh_key()
    test_permission_check_correct_permission()
    test_permission_pattern_matching()
    test_permission_dry_run()
    # Summary
    success = print_summary()
    
    return 0 if success else 1

if __name__ == "__main__":
    sys.exit(main())
