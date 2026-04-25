#!/usr/bin/env sh
#
# One-shot installer for mdtype's git pre-commit hook.
#
# Usage:
#   ./hooks/install.sh                    # install into the current repo
#   ./hooks/install.sh /path/to/repo      # install into a specific repo
#
# Symlinks `hooks/pre-commit` from this checkout into the target repo's
# `.git/hooks/pre-commit`, so updates to the script propagate automatically.

set -eu

target="${1:-.}"

if [ ! -d "$target/.git" ]; then
  echo "install.sh: '$target' is not a git repo (no .git/ directory)." >&2
  exit 1
fi

# Absolute path to the hook script in this checkout.
hook_src="$(cd "$(dirname "$0")" && pwd)/pre-commit"
hook_dst="$target/.git/hooks/pre-commit"

if [ -e "$hook_dst" ] && [ ! -L "$hook_dst" ]; then
  echo "install.sh: $hook_dst exists and is not a symlink." >&2
  echo "Move it aside or pass --force to overwrite (not yet implemented)." >&2
  exit 1
fi

ln -sf "$hook_src" "$hook_dst"
chmod +x "$hook_src"

echo "Installed: $hook_dst -> $hook_src"
echo "Test it with: git commit  (the hook runs against staged .md files)"
