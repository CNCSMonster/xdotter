#!/usr/bin/env python3
"""
xdotter - A simple dotfile manager
Single-file, no dependencies, easy to distribute

Usage:
    python xd.py [COMMAND] [OPTIONS]

Commands:
    deploy      Deploy dotfiles (default)
    undeploy    Remove deployed dotfiles
    new         Create a new xdotter.toml template
    help        Print this help message
    version     Print version

Options:
    -c, --config <FILE>     Specify configuration file [default: xdotter.toml]
    -v, --verbose           Show more information
    -q, --quiet             Do not print any output
    -n, --dry-run           Show what would be done without making changes
    -i, --interactive       Ask for confirmation when unsure
    -f, --force             Force overwrite existing files
    --check-permissions     Check permissions for sensitive files
    --fix-permissions       Fix permissions for sensitive files (implies --check-permissions)

Requires: Python 3.8+
"""

import os
import sys
import stat

# Python version check
if sys.version_info < (3, 8):
    print("Error: Python 3.8+ is required", file=sys.stderr)
    sys.exit(1)

import argparse
from pathlib import Path
from typing import Dict, List, Optional, Tuple

from _vendor.tomli import loads


# ANSI color codes for output
# Yellow for warnings, Red for errors/risks
COLOR_YELLOW = "\033[1;33m"  # Bold yellow
COLOR_RED = "\033[0;31m"     # Red
COLOR_GREEN = "\033[0;32m"   # Green
COLOR_RESET = "\033[0m"      # Reset to default


def get_version() -> str:
    """Get version from environment variable, git tag, or default."""
    # 1. Check environment variable (set by CI during build)
    env_version = os.environ.get("XD_VERSION")
    if env_version:
        return env_version.lstrip("v")
    
    # 2. Try to get from git tag (only if running from source directory)
    try:
        import subprocess
        
        # Determine the git directory (parent of the script or current dir)
        script_path = Path(__file__).parent
        # If running from .pyz, use current working directory instead
        if str(script_path).endswith('.pyz') or not script_path.is_dir():
            git_dir = Path.cwd()
        else:
            git_dir = script_path
        
        result = subprocess.run(
            ["git", "describe", "--tags", "--exact-match"],
            capture_output=True,
            text=True,
            cwd=git_dir
        )
        if result.returncode == 0:
            return result.stdout.strip().lstrip("v")
        
        # Try git describe for dev versions
        result = subprocess.run(
            ["git", "describe", "--tags"],
            capture_output=True,
            text=True,
            cwd=git_dir
        )
        if result.returncode == 0:
            return result.stdout.strip().lstrip("v")
    except (FileNotFoundError, subprocess.SubprocessError):
        pass
    
    # 3. Default version (for development or when git is not available)
    return "0.2.1"


VERSION = get_version()


# Sensitive paths and their required permissions
# Format: path pattern (with ~ support) -> (required_mode, description)
# Supports glob patterns for filenames
SENSITIVE_PATHS: Dict[str, Tuple[int, str]] = {
    # SSH
    "~/.ssh": (0o700, "SSH directory"),
    "~/.ssh/id_rsa": (0o600, "SSH RSA private key"),
    "~/.ssh/id_ed25519": (0o600, "SSH Ed25519 private key"),
    "~/.ssh/id_ecdsa": (0o600, "SSH ECDSA private key"),
    "~/.ssh/id_dsa": (0o600, "SSH DSA private key"),
    "~/.ssh/authorized_keys": (0o600, "SSH authorized keys"),
    "~/.ssh/authorized_keys2": (0o600, "SSH authorized keys (legacy)"),
    "~/.ssh/config": (0o600, "SSH config"),
    "~/.ssh/known_hosts": (0o644, "SSH known hosts"),
    # GPG
    "~/.gnupg": (0o700, "GPG directory"),
    "~/.gnupg/private-keys-v1.d": (0o700, "GPG private keys directory"),
    "~/.gnupg/pubring.kbx": (0o644, "GPG public keyring"),
    "~/.gnupg/pubring.gpg": (0o644, "GPG public keyring (legacy)"),
    "~/.gnupg/secring.gpg": (0o600, "GPG secret keyring (legacy)"),
    "~/.gnupg/gpg.conf": (0o600, "GPG config"),
    # Other sensitive files
    "~/.netrc": (0o600, "Netrc password file"),
    "~/.pgpass": (0o600, "PostgreSQL password file"),
    "~/.my.cnf": (0o600, "MySQL config (may contain passwords)"),
    "~/.pgp/secring.pgp": (0o600, "PGP secret keyring"),
    "~/.config/gnupg": (0o700, "GPG directory (XDG)"),
    "~/.config/gnupg/private-keys-v1.d": (0o700, "GPG private keys directory (XDG)"),
}

# Glob patterns for sensitive files (matched against filename only)
# Format: glob pattern -> (required_mode, description)
SENSITIVE_PATTERNS: List[Tuple[str, int, str]] = [
    # SSH private keys (various names)
    ("id_rsa*", 0o600, "SSH RSA private key"),
    ("id_ed25519*", 0o600, "SSH Ed25519 private key"),
    ("id_ecdsa*", 0o600, "SSH ECDSA private key"),
    ("id_dsa*", 0o600, "SSH DSA private key"),
    ("*_rsa", 0o600, "SSH RSA private key"),
    ("*_ed25519", 0o600, "SSH Ed25519 private key"),
    ("*_ecdsa", 0o600, "SSH ECDSA private key"),
    ("*_dsa", 0o600, "SSH DSA private key"),
    ("*.pem", 0o600, "PEM private key"),
    ("*.key", 0o600, "Private key file"),
    # GPG private keys
    ("*.gpg", 0o600, "GPG file"),
    ("*.asc", 0o600, "ASCII armored key"),
]


class ConfigParser:
    """TOML parser using embedded tomli for robust parsing"""

    @staticmethod
    def parse(content: str) -> Dict:
        """
        Parse TOML content using tomli.
        
        Args:
            content: TOML content as string
            
        Returns:
            Dictionary with 'links' and 'dependencies' keys
        """
        raw_data = loads(content)
        return {
            "links": raw_data.get("links", {}),
            "dependencies": raw_data.get("dependencies", {})
        }


def get_home_dir() -> Path:
    """Get the user's home directory"""
    home = os.path.expanduser("~")
    return Path(home)


def get_required_permission(path: Path) -> Optional[Tuple[int, str]]:
    """
    Get the required permission for a path based on SENSITIVE_PATHS.

    Args:
        path: The path to check (can be absolute or relative to home)

    Returns:
        Tuple of (required_mode, description) if path matches, None otherwise
    """
    import fnmatch
    
    home = get_home_dir()

    # Normalize path to ~ format for matching
    # Use the original path (don't resolve symlinks) for matching
    try:
        # Expand ~ but don't resolve symlinks
        expanded_path = path.expanduser()
        path_str = str(expanded_path)
        home_str = str(home)

        if path_str.startswith(home_str):
            tilde_path = "~" + path_str[len(home_str):]
        else:
            tilde_path = str(path)
    except (OSError, RuntimeError):
        tilde_path = str(path)

    # 1. Direct match in SENSITIVE_PATHS
    if tilde_path in SENSITIVE_PATHS:
        return SENSITIVE_PATHS[tilde_path]

    # 2. Check filename against SENSITIVE_PATTERNS
    filename = expanded_path.name
    for pattern, mode, desc in SENSITIVE_PATTERNS:
        if fnmatch.fnmatch(filename, pattern):
            return (mode, desc)

    # 3. Check for parent directory matches (for files inside sensitive dirs)
    # Use try/except for is_relative_to (Python 3.9+) compatibility
    for sensitive_path, (mode, desc) in SENSITIVE_PATHS.items():
        sensitive_dir = Path(sensitive_path).expanduser()
        try:
            # Python 3.9+ method
            if hasattr(expanded_path, 'is_relative_to'):
                if expanded_path.is_relative_to(sensitive_dir):
                    return (mode, f"inside {desc}")
            else:
                # Python 3.8 fallback: use resolve() and check prefix
                expanded_resolved = expanded_path.resolve()
                sensitive_resolved = sensitive_dir.resolve()
                try:
                    expanded_resolved.relative_to(sensitive_resolved)
                    return (mode, f"inside {desc}")
                except ValueError:
                    pass
        except (OSError, RuntimeError, ValueError):
            continue

    return None


def check_permission(path: Path, required_mode: int, description: str, args) -> Tuple[bool, str]:
    """
    Check if a path has the correct permission.
    
    Args:
        path: Path to check
        required_mode: Required permission mode (e.g., 0o600)
        description: Description of the path
        args: Command line arguments
    
    Returns:
        Tuple of (is_correct, message)
    """
    try:
        # For symlinks, check the target's permission
        actual_path = path.resolve()
        
        if not actual_path.exists():
            return True, f"Path does not exist: {path}"
        
        current_mode = stat.S_IMODE(actual_path.stat().st_mode)
        
        # Check if current permission is more restrictive or equal
        # We check if any extra bits are set that shouldn't be
        extra_bits = current_mode & ~required_mode
        
        if extra_bits == 0:
            return True, f"{COLOR_GREEN}✓{COLOR_RESET} {description}: {path} (permission: {current_mode:03o})"
        else:
            return False, f"{COLOR_RED}✗{COLOR_RESET} {description}: {path} (current: {current_mode:03o}, required: {required_mode:03o})"
            
    except OSError as e:
        return True, f"Cannot check permission for {path}: {e}"


def fix_permission(path: Path, required_mode: int, args) -> Tuple[bool, str]:
    """
    Fix the permission of a path.
    
    Args:
        path: Path to fix
        required_mode: Required permission mode
        args: Command line arguments
    
    Returns:
        Tuple of (success, message)
    """
    try:
        actual_path = path.resolve()
        
        if not actual_path.exists():
            return True, f"Path does not exist: {path}"
        
        if args.dry_run:
            return True, f"Would fix permission for {path} to {required_mode:03o}"
        
        actual_path.chmod(required_mode)
        return True, f"Fixed permission for {path} to {required_mode:03o}"
        
    except OSError as e:
        return False, f"Failed to fix permission for {path}: {e}"


def check_permissions_for_link(link: str, args) -> List[str]:
    """
    Check permissions for a link target if it's a sensitive path.

    Args:
        link: The link path (where symlink will be placed)
        args: Command line arguments

    Returns:
        List of warning/error messages
    """
    messages = []

    home_dir = get_home_dir()
    link_path = Path(link.replace("~", str(home_dir))).expanduser()

    # Check if this link path is a sensitive path
    perm_info = get_required_permission(link_path)

    if perm_info:
        required_mode, description = perm_info

        # Check the source file's permission
        is_correct, msg = check_permission(link_path, required_mode, description, args)
        # Print permission check result directly
        if is_correct:
            log(args, "info", msg)
        else:
            log(args, "warning", msg)
        messages.append(msg)

        # If not correct and fix is requested
        if not is_correct and getattr(args, 'fix_permissions', False):
            success, fix_msg = fix_permission(link_path, required_mode, args)
            messages.append(fix_msg)

    return messages


def log(args, level: str, msg: str):
    """Print log message based on verbosity level"""
    if args.quiet:
        return

    if level == "info" and (args.verbose or not args.quiet):
        print(msg)
    elif level == "debug" and args.verbose:
        print(f"[DEBUG] {msg}")
    elif level == "warning":
        print(f"{COLOR_YELLOW}[WARNING] {msg}{COLOR_RESET}")
    elif level == "error":
        print(f"{COLOR_RED}[ERROR] {msg}{COLOR_RESET}", file=sys.stderr)


def create_symlink(actual_path: str, link: str, args) -> Tuple[bool, Optional[str]]:
    """Create a symlink from link to actual_path"""
    try:
        # Expand and resolve actual path
        actual = Path(actual_path).expanduser().resolve()
        if not actual.exists():
            return False, f"Source path does not exist: {actual}"

        # Expand home directory in link path
        home_dir = get_home_dir()
        link_path = Path(link.replace("~", str(home_dir))).expanduser()

        # Create parent directory if needed
        link_dir = link_path.parent
        if not link_dir.exists():
            if args.dry_run:
                log(args, "debug", f"Would create directory {link_dir}")
            else:
                log(args, "debug", f"Creating directory {link_dir}")
                link_dir.mkdir(parents=True, exist_ok=True)

        # Check if link already exists
        if link_path.exists() or link_path.is_symlink():
            if link_path.is_symlink():
                existing_target = os.readlink(link_path)
                if Path(existing_target).resolve() == actual:
                    log(args, "debug", "Symlink already exists, skipping")
                    return True, None

            # Handle existing file/link
            # Warn if target exists but is not a symlink to this location
            if not link_path.is_symlink():
                log(args, "warning", f"Target exists but is not a symlink: {link_path}")
            
            if args.interactive:
                print(f"Link {link_path} exists, remove it? [y/n] ", end="")
                sys.stdout.flush()
                response = input().strip().lower()
                should_remove = response == "y"
            elif args.force:
                should_remove = True
            else:
                return False, f"Path exists, use --force or --interactive to overwrite: {link_path}"

            if should_remove:
                if args.dry_run:
                    log(args, "debug", f"Would remove {link_path}")
                else:
                    log(args, "debug", f"Removing {link_path}")
                    if link_path.is_dir() and not link_path.is_symlink():
                        import shutil
                        shutil.rmtree(link_path)
                    else:
                        link_path.unlink()
            else:
                log(args, "debug", "Skipping existing link")
                return True, None

        # Create the symlink
        if args.dry_run:
            log(args, "debug", f"Would create symlink {link_path} -> {actual}")
        else:
            log(args, "debug", f"Creating symlink {link_path} -> {actual}")
            os.symlink(actual, link_path)

        return True, None

    except OSError as e:
        return False, f"OS error: {e}"
    except PermissionError as e:
        return False, f"Permission denied: {e}"


def delete_symlink(link: str, args) -> Tuple[bool, Optional[str]]:
    """Delete a symlink"""
    try:
        home_dir = get_home_dir()
        link_path = Path(link.replace("~", str(home_dir))).expanduser()

        if not link_path.exists():
            log(args, "debug", "Link does not exist, skipping")
            return True, None

        if not link_path.is_symlink():
            log(args, "debug", "Not a symlink, skipping")
            return True, None

        # Confirm removal
        if args.interactive:
            print(f"Remove link {link_path}? [y/n] ", end="")
            sys.stdout.flush()
            response = input().strip().lower()
            if response != "y":
                return True, None

        if args.dry_run:
            log(args, "debug", f"Would remove {link_path}")
        else:
            link_path.unlink()
            log(args, "debug", f"Removed {link_path}")

        return True, None

    except OSError as e:
        return False, f"OS error: {e}"
    except PermissionError as e:
        return False, f"Permission denied: {e}"


def deploy_on(config_file: str, args) -> bool:
    """Deploy dotfiles from a config file"""
    log(args, "debug", f"Deploying from {config_file}")

    try:
        with open(config_file, "r", encoding="utf-8") as f:
            content = f.read()
    except FileNotFoundError:
        log(args, "error", f"Config file not found: '{config_file}'")
        return False
    except PermissionError as e:
        log(args, "error", f"Permission denied reading '{config_file}': {e}")
        return False
    except OSError as e:
        log(args, "error", f"Failed to read config '{config_file}': {e}")
        return False

    try:
        config = ConfigParser.parse(content)
    except Exception as e:
        log(args, "error", f"Failed to parse config: {e}")
        return False

    current_dir = Path.cwd()
    success = True

    # Process links
    for actual_path, link in config.get("links", {}).items():
        log(args, "info", f"deploy: {link} -> {actual_path}")
        ok, error = create_symlink(actual_path, link, args)
        if not ok:
            log(args, "error", f"failed to create link: {error}")
            success = False

        # Check permissions for sensitive paths
        if getattr(args, 'check_permissions', False) or getattr(args, 'fix_permissions', False):
            check_permissions_for_link(link, args)

    # Process dependencies
    for dep_name, dep_path in config.get("dependencies", {}).items():
        log(args, "debug", f"dependency: {dep_name}, path: {dep_path}")
        dep_dir = current_dir / dep_path

        try:
            os.chdir(dep_dir)
            log(args, "debug", f"entering {dep_dir}")
            dep_config = dep_dir / "xdotter.toml"
            if dep_config.exists():
                if not deploy_on(str(dep_config), args):
                    success = False
        except FileNotFoundError:
            log(args, "error", f"Dependency directory not found: {dep_dir}")
            success = False
        except OSError as e:
            log(args, "error", f"failed to enter {dep_dir}: {e}")
            success = False
        finally:
            os.chdir(current_dir)
            log(args, "debug", f"leaving {dep_dir}")

    return success


def undeploy_on(config_file: str, args) -> bool:
    """Undeploy dotfiles from a config file"""
    log(args, "debug", f"Undeploying from {config_file}")

    try:
        with open(config_file, "r", encoding="utf-8") as f:
            content = f.read()
    except FileNotFoundError:
        log(args, "error", f"Config file not found: '{config_file}'")
        return False
    except PermissionError as e:
        log(args, "error", f"Permission denied reading '{config_file}': {e}")
        return False
    except OSError as e:
        log(args, "error", f"Failed to read config '{config_file}': {e}")
        return False

    try:
        config = ConfigParser.parse(content)
    except Exception as e:
        log(args, "error", f"Failed to parse config: {e}")
        return False

    current_dir = Path.cwd()
    success = True

    # Process links
    for actual_path, link in config.get("links", {}).items():
        log(args, "info", f"undeploy: {link} -> {actual_path}")
        ok, error = delete_symlink(link, args)
        if not ok:
            log(args, "error", f"failed to delete link: {error}")
            success = False

    # Process dependencies
    for dep_name, dep_path in config.get("dependencies", {}).items():
        dep_dir = current_dir / dep_path

        try:
            os.chdir(dep_dir)
            dep_config = dep_dir / "xdotter.toml"
            if dep_config.exists():
                if not undeploy_on(str(dep_config), args):
                    success = False
        except FileNotFoundError:
            log(args, "error", f"Dependency directory not found: {dep_dir}")
            success = False
        except OSError as e:
            log(args, "error", f"failed to enter {dep_dir}: {e}")
            success = False
        finally:
            os.chdir(current_dir)

    return success


def cmd_new():
    """Create a new xdotter.toml template"""
    template = """# xdotter configuration file
# See: https://github.com/cncsmonster/xdotter

[links]
# Format: "source_path" = "target_link"
# Example:
".config/nvim/init.lua" = "~/.config/nvim/init.lua"
".zshrc" = "~/.zshrc"

[dependencies]
# Format: "name" = "relative_path"
# Example:
# "go" = "testdata/go"
# "nvim" = "config/nvim"
"""

    config_file = "xdotter.toml"
    with open(config_file, "w") as f:
        f.write(template)

    print(f"Created {config_file}")


def print_help():
    """Print help message"""
    help_text = f"""xdotter - A simple dotfile manager (v{VERSION})

USAGE:
    python xd.py [COMMAND] [OPTIONS]

COMMANDS:
    deploy      Deploy dotfiles (default command)
    undeploy    Remove deployed dotfiles
    new         Create a new xdotter.toml template
    help        Print this help message
    version     Print version

OPTIONS:
    -c, --config <FILE>     Specify configuration file [default: xdotter.toml]
    -v, --verbose           Show more information
    -q, --quiet             Do not print any output
    -n, --dry-run           Show what would be done without making changes
    -i, --interactive       Ask for confirmation when unsure
    -f, --force             Force overwrite existing files
    --check-permissions     Check permissions for sensitive files (SSH, GPG, etc.)
    --fix-permissions       Fix permissions for sensitive files

EXAMPLES:
    python xd.py                      Deploy using default xdotter.toml
    python xd.py deploy -v            Deploy with verbose output
    python xd.py deploy --check-permissions   Check sensitive file permissions
    python xd.py deploy --fix-permissions     Fix sensitive file permissions
    python xd.py undeploy -n          Dry-run undeploy
    python xd.py new                  Create new configuration
    python xd.py -c myconfig.toml     Use custom config file

INSTALLATION:
    # Download
    curl -L https://github.com/cncsmonster/xdotter/releases/latest/download/xd.pyz -o ~/.local/bin/xd

    # Make executable
    chmod +x ~/.local/bin/xd

    # Run
    xd --help

LICENSE:
    MIT License - See LICENSE file for details
"""
    print(help_text)


def print_version():
    """Print version"""
    print(f"xdotter {VERSION}")


def main():
    """Main entry point"""
    parser = argparse.ArgumentParser(
        prog="xd",
        description="xdotter - A simple dotfile manager",
        add_help=False,
    )

    parser.add_argument(
        "command",
        nargs="?",
        choices=["deploy", "undeploy", "new", "help", "version"],
        help="Command to execute",
    )
    parser.add_argument(
        "-c", "--config",
        default="xdotter.toml",
        help="Specify configuration file [default: xdotter.toml]"
    )
    parser.add_argument(
        "-v", "--verbose",
        action="store_true",
        help="Show more information"
    )
    parser.add_argument(
        "-q", "--quiet",
        action="store_true",
        help="Do not print any output"
    )
    parser.add_argument(
        "-n", "--dry-run",
        action="store_true",
        help="Show what would be done without making changes"
    )
    parser.add_argument(
        "-i", "--interactive",
        action="store_true",
        help="Ask for confirmation when unsure"
    )
    parser.add_argument(
        "-f", "--force",
        action="store_true",
        help="Force overwrite existing files"
    )
    parser.add_argument(
        "-h", "--help",
        action="store_true",
        help="Print this help message"
    )
    parser.add_argument(
        "--version",
        action="store_true",
        help="Print version"
    )
    parser.add_argument(
        "--check-permissions",
        action="store_true",
        dest="check_permissions",
        help="Check permissions for sensitive files (SSH keys, GPG, etc.)"
    )
    parser.add_argument(
        "--fix-permissions",
        action="store_true",
        dest="fix_permissions",
        help="Fix permissions for sensitive files (implies --check-permissions)"
    )

    args = parser.parse_args()
    
    # --fix-permissions implies --check-permissions
    if args.fix_permissions:
        args.check_permissions = True

    # Handle help and version first
    if args.help or args.command == "help":
        print_help()
        return 0

    if args.version or args.command == "version":
        print_version()
        return 0

    # Handle new command
    if args.command == "new":
        cmd_new()
        return 0

    # Default to deploy if no command specified
    command = args.command or "deploy"

    if command == "deploy":
        if args.dry_run:
            log(args, "info", "Deploying (dry-run)...")
        else:
            log(args, "info", "Deploying...")
        success = deploy_on(args.config, args)
        return 0 if success else 1

    elif command == "undeploy":
        if args.dry_run:
            log(args, "info", "Undeploying (dry-run)...")
        else:
            log(args, "info", "Undeploying...")
        success = undeploy_on(args.config, args)
        return 0 if success else 1

    return 0


if __name__ == "__main__":
    sys.exit(main())
