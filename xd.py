#!/usr/bin/env python3
"""
xdotter - A simple dotfile manager
Single-file, no dependencies, easy to distribute

Usage:
    xd [COMMAND] [OPTIONS]

Commands:
    deploy              Deploy dotfiles (default)
    undeploy            Remove deployed dotfiles
    check-permissions   Check/fix permissions for deployed files
    validate            Validate configuration file syntax
    completion          Generate shell completion scripts
    new                 Create a new xdotter.toml template
    help                Print this help message
    version             Print version

Options:
    -v, --verbose           Show more information
    -q, --quiet             Do not print any output
    -n, --dry-run           Show what would be done without making changes
    -i, --interactive       Ask for confirmation when unsure
    -f, --force             Force overwrite existing files
    --check-permissions     Check permissions for sensitive files
    --fix-permissions       Fix permissions for sensitive files (implies --check-permissions)
    --no-validate           Skip config syntax validation during deploy

Note:
    Shell completion uses vendored argcomplete for automatic generation
    from argparse definition (no manual maintenance required).

Requires: Python 3.8+
"""

import os
import sys
import stat
import json

# Python version check
if sys.version_info < (3, 8):
    print("Error: Python 3.8+ is required", file=sys.stderr)
    sys.exit(1)

import argparse
from pathlib import Path
from typing import Dict, List, Optional, Tuple

from _vendor.tomli import loads, TOMLDecodeError
# Vendored argcomplete for shell completion (optional, for development)
# try:
#     from _vendor.argcomplete import autocomplete
#     HAS_ARGCOMPLETE = True
# except ImportError:
#     HAS_ARGCOMPLETE = False


# ANSI color codes for output
# Yellow for warnings, Red for errors/risks
COLOR_YELLOW = "\033[1;33m"  # Bold yellow
COLOR_RED = "\033[0;31m"     # Red
COLOR_GREEN = "\033[0;32m"   # Green
COLOR_RESET = "\033[0m"      # Reset to default


# Common error suggestions for TOML
TOML_SUGGESTIONS = {
    "Invalid initial character": "TOML 键名不能以特殊字符开头，请用引号包裹",
    "Expected '=' after a key": "TOML 键值对需要使用 = 连接",
    "Unclosed string": "字符串未闭合，请检查引号是否配对",
    "Invalid number": "数字格式错误，检查是否有前导零或非法字符",
    "Invalid value": "无效的值，TOML 支持：字符串、数字、布尔值、日期、数组、表格",
    "Key appears more than once": "键名重复，TOML 不允许重复键名",
    "Unquoted string": "字符串必须用引号包裹（双引号或单引号）",
    "Expected ']' at the end": "表格标题未闭合，缺少 ]",
    "Invalid control character": "不支持控制字符，使用转义序列（如 \\n, \\t）",
}

# Common error suggestions for JSON
JSON_SUGGESTIONS = {
    "Expecting ',' delimiter": "JSON 对象属性之间需要用逗号分隔",
    "Expecting property name": "JSON 键名必须是字符串（用双引号包裹）",
    "Expecting ':' delimiter": "JSON 键值对需要使用冒号分隔",
    "Expecting value": "JSON 值必须是：字符串、数字、布尔值、null、数组或对象",
    "Unterminated string": "字符串未闭合，检查引号是否配对",
    "Invalid control character": "JSON 不支持控制字符，使用转义序列（如 \\n）",
    "Extra data": "JSON 文件只能包含一个顶层值（对象或数组）",
    "Invalid \\escape": "无效的转义序列，JSON 支持：\\\" \\\\ \\/ \\b \\f \\n \\r \\t \\uXXXX",
}


def detect_config_format(filepath: Path) -> Optional[str]:
    """
    Detect configuration file format based on extension.
    
    Args:
        filepath: Path to the configuration file
        
    Returns:
        'toml', 'json', or None if unknown
    """
    suffix = filepath.suffix.lower()
    if suffix == '.toml':
        return 'toml'
    elif suffix == '.json':
        return 'json'
    return None


def get_toml_suggestion(error: TOMLDecodeError) -> Optional[str]:
    """Get suggestion for fixing TOML error"""
    error_msg = str(error).lower()
    for key, suggestion in TOML_SUGGESTIONS.items():
        if key.lower() in error_msg:
            return suggestion
    return None


def get_json_suggestion(error: json.JSONDecodeError) -> Optional[str]:
    """Get suggestion for fixing JSON error"""
    error_msg = error.msg.lower()
    for key, suggestion in JSON_SUGGESTIONS.items():
        if key.lower() in error_msg:
            return suggestion
    return None


def format_toml_error(filepath: Path, content: str, error: TOMLDecodeError) -> str:
    """
    Format TOML error message with context.
    
    Args:
        filepath: Path to the file
        content: File content
        error: TomlDecodeError exception
        
    Returns:
        Formatted error message string
    """
    line = getattr(error, 'lineno', 1)
    col = getattr(error, 'pos', 1)
    
    # Calculate column from position if pos is absolute
    if hasattr(error, 'pos') and error.pos and line > 1:
        lines = content.splitlines()
        if line <= len(lines):
            # Find column by counting characters in the error line
            try:
                line_start = sum(len(lines[i]) + 1 for i in range(line - 1))
                col = error.pos - line_start + 1
            except (IndexError, TypeError):
                col = 1
    
    lines = content.splitlines()
    error_line = lines[line - 1] if line <= len(lines) else ""
    prev_line = lines[line - 2] if line > 1 else ""
    next_line = lines[line] if line < len(lines) else ""
    
    # Build error message
    msg = [
        f"{COLOR_RED}❌ TOML 语法错误{COLOR_RESET}",
        f"",
        f"文件：{filepath}",
        f"错误：{error.msg} (第 {line} 行，第 {col} 列)",
        f"",
        f"第 {line} 行:",
    ]
    
    if prev_line:
        msg.append(f"  {line-1} | {prev_line}")
    msg.append(f"{COLOR_RED}> {line} | {error_line}{COLOR_RESET}")
    msg.append(f"    | {' ' * (col-1)}^")
    if next_line:
        msg.append(f"  {line+1} | {next_line}")
    
    # Add suggestion
    suggestion = get_toml_suggestion(error)
    if suggestion:
        msg.append(f"")
        msg.append(f"{COLOR_YELLOW}提示：{suggestion}{COLOR_RESET}")
    
    return "\n".join(msg)


def format_json_error(filepath: Path, content: str, error: json.JSONDecodeError) -> str:
    """
    Format JSON error message with context.
    
    Args:
        filepath: Path to the file
        content: File content
        error: JSONDecodeError exception
        
    Returns:
        Formatted error message string
    """
    line = error.lineno
    col = error.colno
    
    lines = content.splitlines()
    error_line = lines[line - 1] if line <= len(lines) else ""
    prev_line = lines[line - 2] if line > 1 else ""
    next_line = lines[line] if line < len(lines) else ""
    
    # Build error message
    msg = [
        f"{COLOR_RED}❌ JSON 语法错误{COLOR_RESET}",
        f"",
        f"文件：{filepath}",
        f"错误：{error.msg} (第 {line} 行，第 {col} 列)",
        f"",
        f"第 {line} 行:",
    ]
    
    if prev_line:
        msg.append(f"  {line-1} | {prev_line}")
    msg.append(f"{COLOR_RED}> {line} | {error_line}{COLOR_RESET}")
    msg.append(f"    | {' ' * (col-1)}^")
    if next_line:
        msg.append(f"  {line+1} | {next_line}")
    
    # Add suggestion
    suggestion = get_json_suggestion(error)
    if suggestion:
        msg.append(f"")
        msg.append(f"{COLOR_YELLOW}提示：{suggestion}{COLOR_RESET}")
    
    return "\n".join(msg)


def validate_config(filepath: Path) -> Tuple[bool, str]:
    """
    Validate configuration file syntax.
    
    Args:
        filepath: Path to the configuration file
        
    Returns:
        Tuple of (is_valid, message)
    """
    if not filepath.exists():
        return False, f"File not found: {filepath}"
    
    # Detect format
    fmt = detect_config_format(filepath)
    if fmt is None:
        return False, f"Unknown file format: {filepath.suffix}"
    
    # Read content
    try:
        content = filepath.read_text(encoding='utf-8')
    except OSError as e:
        return False, f"Cannot read file: {e}"
    
    # Validate based on format
    if fmt == 'toml':
        try:
            loads(content)
            return True, f"TOML syntax is valid"
        except TOMLDecodeError as e:
            msg = format_toml_error(filepath, content, e)
            return False, msg
    elif fmt == 'json':
        try:
            json.loads(content)
            return True, f"JSON syntax is valid"
        except json.JSONDecodeError as e:
            msg = format_json_error(filepath, content, e)
            return False, msg
    
    return False, f"Unsupported format: {fmt}"


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
    # Shell configs (affect environment variables and PATH)
    "~/.bashrc": (0o644, "Bash config"),
    "~/.zshrc": (0o644, "Zsh config"),
    "~/.bash_profile": (0o644, "Bash login profile"),
    "~/.profile": (0o644, "Shell profile"),
    "~/.zprofile": (0o644, "Zsh login profile"),
    "~/.zshenv": (0o644, "Zsh environment"),
    "~/.zlogin": (0o644, "Zsh login script"),
    "~/.bash_logout": (0o644, "Bash logout script"),
    # X11 / GUI related (affect graphical session and app launching)
    "~/.xinitrc": (0o755, "X11 initialization script"),
    "~/.xsession": (0o755, "X session script"),
    "~/.xprofile": (0o644, "X profile"),
    "~/.Xauthority": (0o600, "X11 authority file"),
    "~/.Xresources": (0o644, "X resources"),
    "~/.Xdefaults": (0o644, "X defaults"),
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
    # Shell config backups (may contain sensitive data)
    ("*.bashrc", 0o644, "Bash config backup"),
    ("*.zshrc", 0o644, "Zsh config backup"),
    ("*.profile", 0o644, "Shell profile backup"),
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


def check_permissions_for_link(actual_path: Path, link: str, args) -> Tuple[bool, List[str]]:
    """
    Check permissions for the source file if the link target is a sensitive path.

    Args:
        actual_path: The source file path (the actual file, not symlink)
        link: The link path (where symlink will be placed)
        args: Command line arguments

    Returns:
        Tuple of (can_deploy, messages)
        can_deploy is False if permission issue found and not forced
    """
    messages = []
    can_deploy = True

    home_dir = get_home_dir()
    link_path = Path(link.replace("~", str(home_dir))).expanduser()

    # Check if this link path is a sensitive path
    perm_info = get_required_permission(link_path)

    if perm_info:
        required_mode, description = perm_info

        # Check the SOURCE file's permission (not the target, which doesn't exist yet)
        is_correct, msg = check_permission(actual_path, required_mode, description, args)
        # Print permission check result directly
        if is_correct:
            log(args, "info", msg)
        else:
            log(args, "warning", msg)
            # Permission issue found
            if not args.force:
                can_deploy = False
                log(args, "error", f"Skipping {link}: permission issue for {description}")
        messages.append(msg)

        # If not correct and fix is requested
        if not is_correct and getattr(args, 'fix_permissions', False):
            success, fix_msg = fix_permission(actual_path, required_mode, args)
            messages.append(fix_msg)
            if success:
                can_deploy = True  # Can deploy after fixing

    return can_deploy, messages


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

        # Check permissions BEFORE creating symlink (if enabled)
        # This prevents deploying files with wrong permissions to sensitive locations
        if getattr(args, 'check_permissions', False) or getattr(args, 'fix_permissions', False):
            can_deploy, _ = check_permissions_for_link(actual, link, args)
            if not can_deploy:
                return False, f"Permission issue detected, use --force to override"

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
    
    # Validate config syntax before deploying (unless skipped)
    if not getattr(args, 'no_validate', False):
        config_path = Path(config_file)
        if config_path.exists():
            is_valid, msg = validate_config(config_path)
            if not is_valid:
                log(args, "error", msg)
                log(args, "error", "Deployment aborted due to config syntax errors")
                log(args, "info", "Hint: Run 'xd validate' to check config syntax")
                return False

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


def cmd_validate(args) -> bool:
    """
    Validate configuration file syntax.
    
    Args:
        args: Command line arguments
        
    Returns:
        True if all configs are valid, False otherwise
    """
    # Files to check
    if hasattr(args, 'files') and args.files:
        files_to_check = [Path(f) for f in args.files]
    else:
        # Default: check xdotter.toml and xdotter.json
        files_to_check = [Path("xdotter.toml"), Path("xdotter.json")]
    
    all_valid = True
    results = []
    
    for filepath in files_to_check:
        if not filepath.exists():
            # Skip default files that don't exist
            if filepath.name in ['xdotter.toml', 'xdotter.json']:
                continue
            
            log(args, "error", f"File not found: {filepath}")
            all_valid = False
            continue
        
        # Validate
        is_valid, msg = validate_config(filepath)
        
        if is_valid:
            fmt = detect_config_format(filepath).upper()
            log(args, "info", f"{COLOR_GREEN}✓{COLOR_RESET} {filepath} ({fmt}) - Valid syntax")
            results.append((filepath, True))
        else:
            log(args, "error", msg)
            all_valid = False
            results.append((filepath, False))
    
    # Summary
    if not args.quiet and results:
        total = len(results)
        valid = sum(1 for _, v in results if v)
        invalid = total - valid
        
        log(args, "info", "")
        if invalid == 0:
            log(args, "info", f"{COLOR_GREEN}✓ All {total} configuration file(s) have valid syntax{COLOR_RESET}")
        else:
            log(args, "warning", f"{COLOR_RED}✗ {invalid}/{total} configuration file(s) have syntax errors{COLOR_RESET}")
    
    return all_valid


def cmd_check_perms(args) -> bool:
    """
    Check and optionally fix permissions for deployed symlinks.

    This command checks the permissions of files that have been deployed
    via symlinks to sensitive locations (SSH keys, shell configs, etc.).
    """
    config_file = "xdotter.toml"
    log(args, "debug", f"Checking permissions from {config_file}")

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
    
    home_dir = get_home_dir()
    success = True
    checked_count = 0
    fixed_count = 0
    
    # Check permissions for all links
    for actual_path, link in config.get("links", {}).items():
        link_path = Path(link.replace("~", str(home_dir))).expanduser()
        
        # Only check if symlink exists
        if not link_path.is_symlink():
            log(args, "debug", f"Skipping {link}: not a symlink")
            continue
        
        # Get the target file (resolve symlink)
        try:
            target_path = link_path.resolve()
        except OSError as e:
            log(args, "error", f"Cannot resolve {link}: {e}")
            success = False
            continue
        
        # Check if this is a sensitive path
        perm_info = get_required_permission(link_path)
        
        if perm_info:
            required_mode, description = perm_info
            checked_count += 1
            
            # Check permission
            is_correct, msg = check_permission(target_path, required_mode, description, args)
            
            if is_correct:
                log(args, "info", msg)
            else:
                log(args, "warning", msg)
                
                # Fix if requested
                if getattr(args, 'fix_permissions', False):
                    if args.dry_run:
                        log(args, "info", f"Would fix permission for {target_path}")
                    else:
                        ok, fix_msg = fix_permission(target_path, required_mode, args)
                        log(args, "info", fix_msg)
                        if ok:
                            fixed_count += 1
                        else:
                            success = False
                else:
                    success = False  # Report failure if not fixing
    
    # Summary
    if not args.quiet:
        log(args, "info", f"Checked {checked_count} sensitive file(s)")
        if getattr(args, 'fix_permissions', False):
            log(args, "info", f"Fixed {fixed_count} file(s)")
    
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


def cmd_completion(args) -> int:
    """
    Generate shell completion scripts.

    Usage:
        xd completion bash
        xd completion zsh
        xd completion fish

    Uses simplified completion scripts designed for eval usage.
    """
    if not hasattr(args, 'shell') or not args.shell:
        log(args, "error", "Shell name required")
        log(args, "info", "Usage: xd completion <bash|zsh|fish>")
        return 1

    shell = args.shell.lower()

    if shell == 'bash':
        print(BASH_EVAL_COMPLETION)
        return 0
    elif shell == 'zsh':
        print(ZSH_EVAL_COMPLETION)
        return 0
    elif shell == 'fish':
        print(FISH_COMPLETION)
        return 0
    else:
        log(args, "error", f"Unsupported shell: '{shell}'")
        log(args, "info", "Supported shells: bash, zsh, fish")
        return 1


# Simplified completion scripts designed for eval usage
# These scripts work better with eval "$(xd completion <shell>)" pattern
# Note: We use $(which xd) to ensure we call the correct executable

BASH_EVAL_COMPLETION = r'''
_xd_completion() {
    local IFS=$'\013'
    local COMPLETIONS
    local XD_PATH
    XD_PATH=$(which xd 2>/dev/null) || XD_PATH="xd"
    COMPLETIONS=($(IFS="$IFS" \
        COMP_LINE="$COMP_LINE" \
        COMP_POINT="$COMP_POINT" \
        COMP_TYPE="$COMP_TYPE" \
        COMP_WORDBREAKS="$COMP_WORDBREAKS" \
        _ARGCOMPLETE=1 \
        _ARGCOMPLETE_SHELL="bash" \
        _ARGCOMPLETE_SUPPRESS_SPACE=1 \
        _ARGCOMPLETE_IFS=$'\013' \
        "$XD_PATH" 8>&1 9>&2 1>/dev/null 2>&1))
    if [[ ${#COMPLETIONS[@]} -gt 0 ]]; then
        COMPREPLY=("${COMPLETIONS[@]}")
        if [[ "${COMPREPLY[-1]}" =~ [=/:]$ ]]; then
            compopt -o nospace 2>/dev/null
        fi
    fi
}
complete -F _xd_completion xd
'''

ZSH_EVAL_COMPLETION = r'''
# Load completion system if not already loaded
autoload -Uz compinit && compinit 2>/dev/null || true

_xd_completion() {
    local -a completions
    local XD_PATH
    XD_PATH=$(which xd 2>/dev/null) || XD_PATH="xd"
    
    # Capture completions from xd in a subshell with proper environment
    completions=("${(@f)$(
        export _ARGCOMPLETE=1
        export _ARGCOMPLETE_SHELL="zsh"
        export _ARGCOMPLETE_SUPPRESS_SPACE=1
        export _ARGCOMPLETE_IFS=$'\n'
        export COMP_LINE="$BUFFER"
        export COMP_POINT="$CURSOR"
        "$XD_PATH" 8>&1 9>&2 1>/dev/null 2>&1
    )}")
    
    if [[ ${#completions[@]} -gt 0 && -n "${completions[1]}" ]]; then
        local -a replies
        local comp
        # Parse "completion:description" format
        for comp in "${completions[@]}"; do
            if [[ "$comp" == *:* ]]; then
                # Has description - use compadd with -l and -d
                replies+=("${comp%%:*}")
            else
                replies+=("$comp")
            fi
        done
        compadd -a replies
    fi
}
compdef _xd_completion xd
'''

FISH_COMPLETION = r'''function __fish_xd_complete
    set -l XD_PATH (which xd 2>/dev/null)
    if test -z "$XD_PATH"
        set XD_PATH xd
    end
    set -lx _ARGCOMPLETE 1
    set -lx _ARGCOMPLETE_SHELL fish
    set -lx _ARGCOMPLETE_IFS \n
    set -lx COMP_LINE (commandline -p)
    set -lx COMP_POINT (string length (commandline -cp))
    "$XD_PATH" 8>&1 9>&2 1>/dev/null 2>&1
end
complete -c xd -a '(__fish_xd_complete)' -f
'''


def print_help():
    """Print help message"""
    help_text = f"""xdotter - A simple dotfile manager (v{VERSION})

USAGE:
    xd [COMMAND] [OPTIONS]

COMMANDS:
    deploy              Deploy dotfiles (default command)
    undeploy            Remove deployed dotfiles
    check-permissions   Check/fix permissions for deployed files
    validate            Validate configuration file syntax
    completion          Generate shell completion scripts
    new                 Create a new xdotter.toml template
    help                Print this help message
    version             Print version

OPTIONS:
    -v, --verbose           Show more information
    -q, --quiet             Do not print any output
    -n, --dry-run           Show what would be done without making changes
    -i, --interactive       Ask for confirmation when unsure
    -f, --force             Force overwrite existing files
    --check-permissions     Check permissions for sensitive files (SSH, GPG, etc.)
    --fix-permissions       Fix permissions for sensitive files
    --no-validate           Skip config syntax validation during deploy

EXAMPLES:
    xd                            Deploy using xdotter.toml
    xd deploy -v                  Deploy with verbose output
    xd deploy --check-permissions Check sensitive file permissions
    xd deploy --fix-permissions   Fix sensitive file permissions
    xd validate                   Validate configuration file syntax
    xd completion bash            Generate Bash completion script
    xd check-permissions --fix-permissions  Fix permissions for deployed files
    xd undeploy -n                Dry-run undeploy
    xd new                        Create new configuration

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
        choices=["deploy", "undeploy", "check-permissions", "validate", "completion", "new", "help", "version"],
        help="Command to execute",
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
        "--no-validate",
        action="store_true",
        dest="no_validate",
        help="Skip config syntax validation during deploy"
    )
    parser.add_argument(
        "--shell",
        help="Shell name for completion command (bash|zsh|fish)"
    )
    parser.add_argument(
        "files",
        nargs="*",
        help="Configuration files to validate (for validate command)"
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

    # Enable argcomplete autocomplete (if available)
    # This is automatically activated when COMP_LINE env var is set by bash
    try:
        from _vendor.argcomplete import autocomplete
        autocomplete(parser)
    except (ImportError, TypeError):
        # argcomplete not vendored or not running in completion context
        pass

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

    # Handle completion command
    if args.command == "completion":
        # Support both --shell flag and positional argument for backward compatibility
        if hasattr(args, 'shell') and args.shell:
            args.shell = args.shell
        elif hasattr(args, 'files') and args.files:
            args.shell = args.files[0]
        return cmd_completion(args)

    # Handle validate command
    if args.command == "validate":
        if args.dry_run:
            log(args, "info", "Validating configuration (dry-run)...")
        else:
            log(args, "info", "Validating configuration...")
        success = cmd_validate(args)
        return 0 if success else 1

    # Handle check-permissions command
    if args.command == "check-permissions":
        if args.dry_run:
            log(args, "info", "Checking permissions (dry-run)...")
        else:
            log(args, "info", "Checking permissions...")
        success = cmd_check_perms(args)
        return 0 if success else 1

    # Default to deploy if no command specified
    command = args.command or "deploy"

    if command == "deploy":
        if args.dry_run:
            log(args, "info", "Deploying (dry-run)...")
        else:
            log(args, "info", "Deploying...")
        success = deploy_on("xdotter.toml", args)
        return 0 if success else 1

    elif command == "undeploy":
        if args.dry_run:
            log(args, "info", "Undeploying (dry-run)...")
        else:
            log(args, "info", "Undeploying...")
        success = undeploy_on("xdotter.toml", args)
        return 0 if success else 1

    return 0


if __name__ == "__main__":
    sys.exit(main())
