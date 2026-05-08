"""Global fixtures for SPEC-driven xdotter tests.

All tests in tests/spec/ interact with the xdotter binary purely via CLI.
The binary path is resolved via XDOTTER_BIN env var, falling back to the
cargo-built binary. Each test gets its own isolated temp repo and home dir.

SPEC terminology used in test names and docstrings:
- "规划跳过" (planned skip) — a link that the planning stage decided not to
  process (e.g. non-empty directory). No filesystem operation is attempted.
  This is NOT an "apply-stage failure" per SPEC.
- "应用阶段失败" (apply-stage failure) — a filesystem operation (create symlink,
  remove file, change permissions) actually executed and returned an error.
  Per SPEC, this must stop all subsequent operations.
- "可恢复冲突" (recoverable conflict) — a condition the current mode (default/
  force/interactive) chooses not to handle. May be skipped or auto-resolved
  depending on the mode.
"""

from __future__ import annotations

import dataclasses
import os
import shutil
import subprocess
import tempfile
from pathlib import Path

import pytest

# ---------------------------------------------------------------------------
# Data class for subprocess results
# ---------------------------------------------------------------------------

@dataclasses.dataclass(frozen=True)
class Result:
    """Captured output from an xdotter subprocess invocation."""
    code: int
    stdout: str
    stderr: str

# ---------------------------------------------------------------------------
# Atomic counter for unique temp paths
# ---------------------------------------------------------------------------

_counter = 0

def _next_id() -> int:
    global _counter
    _counter += 1
    return _counter

# ---------------------------------------------------------------------------
# Binary resolution
# ---------------------------------------------------------------------------

def _resolve_binary() -> Path:
    """Return the xdotter binary path.

    Priority:
    1. XDOTTER_BIN environment variable
    2. cargo build debug binary at project root target/debug/xd
    """
    env_path = os.environ.get("XDOTTER_BIN")
    if env_path:
        p = Path(env_path)
        if not p.exists():
            raise FileNotFoundError(f"XDOTTER_BIN={env_path} does not exist")
        return p

    # Fallback: assume cargo debug build exists at project root
    project_root = Path(__file__).resolve().parent.parent.parent
    binary = project_root / "target" / "debug" / "xd"
    if not binary.exists():
        raise FileNotFoundError(
            f"No xdotter binary found at {binary}. "
            "Run `cargo build` first or set XDOTTER_BIN."
        )
    return binary

_XD_BIN: Path | None = None

def _xd_bin() -> Path:
    global _XD_BIN
    if _XD_BIN is None:
        _XD_BIN = _resolve_binary()
    return _XD_BIN

# ---------------------------------------------------------------------------
# Fixtures
# ---------------------------------------------------------------------------

@pytest.fixture(scope="session")
def xd_binary() -> Path:
    """Resolved path to the xdotter binary."""
    return _xd_bin()


@pytest.fixture
def tmp_repo(tmp_path: Path) -> Path:
    """Return an empty temp directory to use as a dotfile repo root."""
    repo = tmp_path / "repo"
    repo.mkdir()
    return repo


@pytest.fixture
def unique_home(tmp_path: Path) -> Path:
    """Return an empty temp directory to use as HOME."""
    home = tmp_path / "home"
    home.mkdir()
    return home


@pytest.fixture
def run_xd(xd_binary: Path) -> callable:
    """Factory that returns a function to invoke the xdotter binary.

    Usage::

        def test_something(run_xd, tmp_repo, unique_home):
            repo = tmp_repo
            home = unique_home
            result = run_xd(["deploy"], cwd=repo, home=home)
            assert result.code == 0

    Parameters to the returned function:
        args: list of CLI arguments (e.g. ["deploy", "--force"])
        cwd: Path to the repo root (where xdotter.toml lives)
        home: Path to use as HOME
        stdin: optional stdin content to pipe in (for interactive mode)
        extra_env: dict of additional environment variables

    Returns a Result(code, stdout, stderr).
    """
    def _run(
        args: list[str],
        cwd: Path,
        home: Path,
        *,
        stdin: str | None = None,
        extra_env: dict[str, str] | None = None,
    ) -> Result:
        env: dict[str, str] = {
            "HOME": str(home),
            "PATH": os.environ.get("PATH", ""),
        }
        if extra_env:
            env.update(extra_env)

        proc = subprocess.run(
            [str(xd_binary), *args],
            cwd=str(cwd),
            env=env,
            input=stdin.encode() if stdin else None,
            capture_output=True,
            timeout=30,
        )
        return Result(
            code=proc.returncode,
            stdout=proc.stdout.decode(errors="replace"),
            stderr=proc.stderr.decode(errors="replace"),
        )
    return _run
