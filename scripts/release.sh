#!/usr/bin/env bash
# Build a stripped release binary for this machine into dist/.
# Publishing is ./scripts/publish.sh
set -euo pipefail

cd "$(dirname "$0")/.."

if [[ -n "${1:-}" ]]; then
  echo "Unknown argument: $1" >&2
  echo "Usage: ./scripts/release.sh" >&2
  echo "To copy the build into the releases repo and push it: ./scripts/publish.sh" >&2
  exit 1
fi

if [[ -f "${HOME}/.cargo/env" ]]; then
  # shellcheck disable=SC1091
  source "${HOME}/.cargo/env"
fi

VERSION="$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)"
NAME="pencil-api-local"
HOST="$(rustc -vV | awk '/^host:/{print $2}')"

case "${HOST}" in
  x86_64-unknown-linux-gnu) platform="linux-x64" ;;
  x86_64-unknown-linux-musl) platform="linux-x64-musl" ;;
  aarch64-unknown-linux-gnu) platform="linux-arm64" ;;
  aarch64-unknown-linux-musl) platform="linux-arm64-musl" ;;
  x86_64-apple-darwin) platform="darwin-x64" ;;
  aarch64-apple-darwin) platform="darwin-arm64" ;;
  x86_64-pc-windows-msvc | x86_64-pc-windows-gnu) platform="windows-x64" ;;
  aarch64-pc-windows-msvc) platform="windows-arm64" ;;
  *) platform="${HOST}" ;;
esac

echo "Building ${NAME} ${VERSION} for ${platform}"
cargo build --release

mkdir -p dist
rm -f dist/${NAME} dist/${NAME}-* dist/SHA256SUMS

binary="target/release/${NAME}"
if [[ "${platform}" == windows-* ]]; then
  binary="${binary}.exe"
fi

outfile="dist/${NAME}-${VERSION}-${platform}"
if [[ "${platform}" == windows-* ]]; then
  outfile="${outfile}.exe"
fi

cp -a "${binary}" "${outfile}"
cp -a "${binary}" "dist/${NAME}"

if command -v sha256sum >/dev/null 2>&1; then
  (cd dist && sha256sum "${outfile#dist/}" > SHA256SUMS)
else
  (cd dist && shasum -a 256 "${outfile#dist/}" > SHA256SUMS)
fi

echo "Built ${outfile}"
