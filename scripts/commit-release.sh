#!/usr/bin/env bash
# Copy versioned binaries into releases/, commit, and push main.
#   ./scripts/commit-release.sh <artifact-dir>
set -euo pipefail

cd "$(dirname "$0")/.."

artifact_dir="${1:-}"
if [[ -z "${artifact_dir}" || ! -d "${artifact_dir}" ]]; then
  echo "Usage: ./scripts/commit-release.sh <artifact-dir>" >&2
  exit 1
fi

files=()
while IFS= read -r -d '' file; do
  files+=("${file}")
done < <(find "${artifact_dir}" -type f -name 'pencil-api-local-*' -print0 | sort -z)
if [[ ${#files[@]} -eq 0 ]]; then
  echo "No release binaries found in ${artifact_dir}" >&2
  exit 1
fi

export GIT_AUTHOR_NAME="github-actions[bot]"
export GIT_AUTHOR_EMAIL="41898282+github-actions[bot]@users.noreply.github.com"
export GIT_COMMITTER_NAME="${GIT_AUTHOR_NAME}"
export GIT_COMMITTER_EMAIL="${GIT_AUTHOR_EMAIL}"

sha="${GITHUB_SHA:-$(git rev-parse HEAD)}"
git fetch origin main
git checkout --detach "${sha}"

if git merge-base --is-ancestor HEAD origin/main \
  && [[ "$(git rev-parse HEAD)" != "$(git rev-parse origin/main)" ]] \
  && git diff --name-only HEAD..origin/main | grep -qv '^releases/'; then
  echo "origin/main has newer source than ${sha}. Skipping publish."
  exit 0
fi

if ! git merge-base --is-ancestor origin/main HEAD; then
  git merge origin/main -m "Merge main into release." -X ours
fi

version="$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)"
mkdir -p releases
for file in "${files[@]}"; do
  cp -a "${file}" "releases/$(basename "${file}")"
done
(
  cd releases
  sha256sum pencil-api-local-* | LC_ALL=C sort -k2 > SHA256SUMS
)

git add -- releases
if git diff --cached --quiet; then
  echo "Release binaries are unchanged."
  exit 0
fi

git commit -m "Publish pencil-api-local ${version}."
git push origin HEAD:main
echo "Pushed pencil-api-local ${version} to releases/"
