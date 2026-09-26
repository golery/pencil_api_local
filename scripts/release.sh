#!/usr/bin/env bash
# Build standalone pencil-api-local binaries.
#   ./scripts/release.sh            write dist/ for every target
#   ./scripts/release.sh --publish  also create a GitHub release for tag v<version>
set -euo pipefail

cd "$(dirname "$0")/.."

VERSION="$(bun -e 'const pkg = await Bun.file("package.json").json(); console.log(pkg.version)')"
NAME="pencil-api-local"
OUT="dist"
publish=0

if [[ "${1:-}" == "--publish" ]]; then
  publish=1
elif [[ -n "${1:-}" ]]; then
  echo "Unknown argument: $1" >&2
  echo "Usage: ./scripts/release.sh [--publish]" >&2
  exit 1
fi

targets=(
  bun-linux-x64
  bun-linux-arm64
  bun-linux-x64-musl
  bun-darwin-x64
  bun-darwin-arm64
  bun-windows-x64
)

mkdir -p "$OUT"
rm -f "$OUT"/${NAME}-* "$OUT"/SHA256SUMS

for target in "${targets[@]}"; do
  platform="${target#bun-}"
  outfile="$OUT/${NAME}-${VERSION}-${platform}"
  if [[ "$platform" == windows-* ]]; then
    outfile="${outfile}.exe"
  fi
  echo "Building ${outfile}"
  bun build src/cli.ts --compile --target="$target" --outfile="$outfile"
done

if command -v sha256sum >/dev/null 2>&1; then
  (cd "$OUT" && sha256sum ${NAME}-* > SHA256SUMS)
else
  (cd "$OUT" && shasum -a 256 ${NAME}-* > SHA256SUMS)
fi

echo "Built ${NAME} ${VERSION} into ${OUT}/"

if [[ "$publish" -ne 1 ]]; then
  exit 0
fi

tag="${GITHUB_REF_NAME:-}"
if [[ -z "$tag" ]]; then
  tag="$(git describe --tags --exact-match 2>/dev/null || true)"
fi
if [[ "$tag" != "v${VERSION}" ]]; then
  echo "Refusing to publish: expected tag v${VERSION}, got '${tag:-none}'." >&2
  echo "Commit the version bump, then: git tag v${VERSION} && git push origin v${VERSION}" >&2
  exit 1
fi

notes="$(mktemp)"
trap 'rm -f "$notes"' EXIT
cat >"$notes" <<EOF
Standalone CLI for the local Pencil vault API (${VERSION}).

Download the binary for your platform, then run it to start the server (default http://127.0.0.1:8300):

\`\`\`
./${NAME}-${VERSION}-linux-x64
./${NAME}-${VERSION}-linux-x64 --port 8300 --books-file ./data/books.json
\`\`\`

A \`.env\` file in the working directory is loaded automatically. \`PORT\`, \`BOOKS_FILE\`, and \`CORS_ORIGINS\` still apply.
EOF

if gh release view "$tag" >/dev/null 2>&1; then
  gh release upload "$tag" "$OUT"/${NAME}-* "$OUT"/SHA256SUMS --clobber
else
  gh release create "$tag" "$OUT"/${NAME}-* "$OUT"/SHA256SUMS \
    --title "${NAME} ${VERSION}" \
    --notes-file "$notes"
fi

echo "Published GitHub release ${tag}"
