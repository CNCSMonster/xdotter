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
"""

import os
import sys
import argparse
from pathlib import Path
from typing import Dict, Optional, Tuple

# Add vendor directory for tomli
_vendor_path = Path(__file__).parent / '_vendor'
if str(_vendor_path) not in sys.path:
    sys.path.insert(0, str(_vendor_path))

from tomli import loads

VERSION = "0.2.0"


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


def log(args, level: str, msg: str):
    """Print log message based on verbosity level"""
    if args.quiet:
        return

    if level == "info" and (args.verbose or not args.quiet):
        print(msg)
    elif level == "debug" and args.verbose:
        print(f"[DEBUG] {msg}")
    elif level == "error":
        print(f"[ERROR] {msg}", file=sys.stderr)


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
        if not link_dir.exists() and not args.dry_run:
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
            if args.interactive:
                print(f"Link {link_path} exists, remove it? [y/n] ", end="")
                sys.stdout.flush()
                response = input().strip().lower()
                should_remove = response == "y"
            elif args.force:
                should_remove = True
            else:
                return False, f"Path exists, use --force or --interactive to overwrite: {link_path}"

            if should_remove and not args.dry_run:
                log(args, "debug", f"Removing {link_path}")
                if link_path.is_dir() and not link_path.is_symlink():
                    import shutil
                    shutil.rmtree(link_path)
                else:
                    link_path.unlink()
            elif not should_remove:
                log(args, "debug", "Skipping existing link")
                return True, None
            else:
                return False, f"Cannot remove: {link_path}"

        # Create the symlink
        if not args.dry_run:
            log(args, "debug", f"Creating symlink {link_path} -> {actual}")
            os.symlink(actual, link_path)

        return True, None

    except Exception as e:
        return False, str(e)


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

        if not args.dry_run:
            link_path.unlink()
            log(args, "debug", f"Removed {link_path}")

        return True, None

    except Exception as e:
        return False, str(e)


def deploy_on(config_file: str, args) -> bool:
    """Deploy dotfiles from a config file"""
    log(args, "debug", f"Deploying from {config_file}")

    try:
        with open(config_file, "r") as f:
            content = f.read()
    except Exception as e:
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
        if not args.dry_run:
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
        except Exception as e:
            log(args, "error", f"failed to enter {dep_dir}: {e}")
            continue
        finally:
            os.chdir(current_dir)
            log(args, "debug", f"leaving {dep_dir}")

    return success


def undeploy_on(config_file: str, args) -> bool:
    """Undeploy dotfiles from a config file"""
    log(args, "debug", f"Undeploying from {config_file}")

    try:
        with open(config_file, "r") as f:
            content = f.read()
    except Exception as e:
        log(args, "error", f"Failed to read config '{config_file}': {e}")
        return False

    try:
        config = ConfigParser.parse(content)
    except Exception as e:
        log(args, "error", f"Failed to parse config: {e}")
        return False

    current_dir = Path.cwd()
    success = True

    # Process links (in reverse order would be better, but keeping simple)
    for actual_path, link in config.get("links", {}).items():
        log(args, "info", f"undeploy: {link} -> {actual_path}")
        if not args.dry_run:
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
        except Exception as e:
            log(args, "error", f"failed to enter {dep_dir}: {e}")
            continue
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

EXAMPLES:
    python xd.py                      Deploy using default xdotter.toml
    python xd.py deploy -v            Deploy with verbose output
    python xd.py undeploy -n          Dry-run undeploy
    python xd.py new                  Create new configuration
    python xd.py -c myconfig.toml     Use custom config file

INSTALLATION:
    # Quick install (downloads to ~/.local/bin/xd)
    curl -sSL https://raw.githubusercontent.com/cncsmonster/xdotter/main/install.sh | bash

    # Or download and run directly
    curl -sSL https://raw.githubusercontent.com/cncsmonster/xdotter/main/xd.py -o xd.py
    chmod +x xd.py
    ./xd.py --help

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

    args = parser.parse_args()

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
        log(args, "info", "Deploying...")
        success = deploy_on(args.config, args)
        return 0 if success else 1

    elif command == "undeploy":
        log(args, "info", "Undeploying...")
        success = undeploy_on(args.config, args)
        return 0 if success else 1

    return 0


if __name__ == "__main__":
    sys.exit(main())
