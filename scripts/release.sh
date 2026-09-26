#!/usr/bin/env bash
# Build standalone pencil-api-local binaries into dist/.
# Publishing is ./scripts/publish.sh
set -euo pipefail

cd "$(dirname "$0")/.."

if [[ -n "${1:-}" ]]; then
  echo "Unknown argument: $1" >&2
  echo "Usage: ./scripts/release.sh" >&2
  echo "To copy the build into the releases repo and push it: ./scripts/publish.sh" >&2
  exit 1
fi

VERSION="$(bun -e 'const pkg = await Bun.file("package.json").json(); console.log(pkg.version)')"
NAME="pencil-api-local"
OUT="dist"

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
