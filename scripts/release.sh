#!/usr/bin/env bash
# Fast-forward main to the current branch and push it.
# The push starts .github/workflows/release.yml, which commits releases/.
#   ./scripts/release.sh
set -euo pipefail

cd "$(dirname "$0")/.."

LAND_WORKTREE=""

cleanup_land_worktree() {
  if [[ -n "${LAND_WORKTREE}" ]]; then
    git worktree remove --force "${LAND_WORKTREE}" >/dev/null 2>&1 || true
    LAND_WORKTREE=""
  fi
}

# Merge the current branch into main in another worktree so this checkout stays put.
land_on_main() {
  local branch
  branch="$(git rev-parse --abbrev-ref HEAD)"
  if [[ "${branch}" == "HEAD" ]]; then
    echo "Detached HEAD. Check out a branch before releasing." >&2
    exit 1
  fi
  if ! git diff --quiet || ! git diff --cached --quiet; then
    echo "Commit changes before releasing." >&2
    exit 1
  fi

  git fetch origin main

  if [[ "${branch}" == "main" ]]; then
    git merge --ff-only origin/main
    git push origin main
    return
  fi

  LAND_WORKTREE="$(mktemp -d "${TMPDIR:-/tmp}/pencil-api-local-main.XXXXXX")"
  rmdir "${LAND_WORKTREE}"
  git worktree add "${LAND_WORKTREE}" main
  trap cleanup_land_worktree EXIT

  if ! git -C "${LAND_WORKTREE}" merge --ff-only origin/main; then
    echo "main has diverged from origin/main." >&2
    exit 1
  fi
  if ! git -C "${LAND_WORKTREE}" merge --ff-only "${branch}"; then
    echo "Cannot fast-forward main to ${branch}. Update ${branch} with main first." >&2
    exit 1
  fi
  git -C "${LAND_WORKTREE}" push origin main

  cleanup_land_worktree
  trap - EXIT
}

land_on_main
echo "Pushed main. The release workflow will commit releases/."
