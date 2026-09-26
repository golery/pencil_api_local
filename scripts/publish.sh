#!/usr/bin/env bash
# Build standalone binaries, copy them into the releases repo, then commit and push.
#   ./scripts/publish.sh
# Override the checkout with RELEASES_REPO=/path/to/releases
set -euo pipefail

cd "$(dirname "$0")/.."

export PATH="${HOME}/.local/bin:${PATH}"

RELEASES_REPO="${RELEASES_REPO:-/home/hly/repos/releases}"
NAME="pencil-api-local"

if [[ ! -d "${RELEASES_REPO}/.git" ]]; then
  echo "Releases repo not found: ${RELEASES_REPO}" >&2
  exit 1
fi

if ! command -v git-lfs >/dev/null 2>&1; then
  echo "git-lfs is required. Binaries over 100MB cannot be pushed to GitHub without it." >&2
  exit 1
fi

./scripts/release.sh

VERSION="$(bun -e 'const pkg = await Bun.file("package.json").json(); console.log(pkg.version)')"
DEST="${RELEASES_REPO}/pencil_api_local"
mkdir -p "${DEST}"

shopt -s nullglob
artifacts=(dist/${NAME}-${VERSION}-*)
shopt -u nullglob
if [[ ${#artifacts[@]} -eq 0 ]]; then
  echo "No release artifacts found for ${NAME} ${VERSION}" >&2
  exit 1
fi

cp -a "${artifacts[@]}" dist/SHA256SUMS "${DEST}/"

attr="${RELEASES_REPO}/.gitattributes"
lfs_line="pencil_api_local/${NAME}-* filter=lfs diff=lfs merge=lfs -text"
if [[ ! -f "${attr}" ]] || ! grep -qxF "${lfs_line}" "${attr}"; then
  echo "${lfs_line}" >> "${attr}"
fi

cd "${RELEASES_REPO}"
git lfs install --local >/dev/null
git add .gitattributes pencil_api_local

if git diff --cached --quiet; then
  echo "Releases repo already contains ${NAME} ${VERSION}"
  exit 0
fi

git commit -m "$(cat <<EOF
Publish ${NAME} ${VERSION}.

EOF
)"

git push -u origin HEAD
echo "Pushed ${NAME} ${VERSION} to ${RELEASES_REPO}"
