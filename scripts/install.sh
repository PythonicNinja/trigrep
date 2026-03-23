#!/usr/bin/env bash
set -euo pipefail

TRIGREP_REPO="${TRIGREP_REPO:-PythonicNinja/trigrep}"
TRIGREP_VERSION="${TRIGREP_VERSION:-}"
TRIGREP_INSTALL_DIR="${TRIGREP_INSTALL_DIR:-}"

require_tool() {
  if ! command -v "$1" >/dev/null 2>&1; then
    echo "error: required tool not found: $1" >&2
    exit 1
  fi
}

sha256_file() {
  local file="$1"

  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$file" | awk '{print $1}'
    return
  fi

  if command -v shasum >/dev/null 2>&1; then
    shasum -a 256 "$file" | awk '{print $1}'
    return
  fi

  echo "error: no SHA256 tool found (need sha256sum or shasum)" >&2
  exit 1
}

resolve_latest_version() {
  local latest_url="https://github.com/${TRIGREP_REPO}/releases/latest"
  local final_url

  final_url="$(curl -fsSLI -o /dev/null -w '%{url_effective}' "$latest_url")"
  basename "$final_url"
}

detect_target() {
  local os
  local arch

  os="$(uname -s)"
  arch="$(uname -m)"

  case "$os" in
    Linux)
      case "$arch" in
        x86_64)
          echo "x86_64-unknown-linux-gnu"
          ;;
        aarch64|arm64)
          echo "aarch64-unknown-linux-gnu"
          ;;
        *)
          echo "error: unsupported Linux architecture: $arch" >&2
          exit 1
          ;;
      esac
      ;;
    Darwin)
      case "$arch" in
        x86_64)
          echo "x86_64-apple-darwin"
          ;;
        arm64|aarch64)
          echo "aarch64-apple-darwin"
          ;;
        *)
          echo "error: unsupported macOS architecture: $arch" >&2
          exit 1
          ;;
      esac
      ;;
    *)
      echo "error: unsupported OS: $os (this installer supports macOS and Linux)" >&2
      exit 1
      ;;
  esac
}

select_install_dir() {
  if [ -n "$TRIGREP_INSTALL_DIR" ]; then
    echo "$TRIGREP_INSTALL_DIR"
    return
  fi

  if [ -d "/usr/local/bin" ] && [ -w "/usr/local/bin" ]; then
    echo "/usr/local/bin"
    return
  fi

  echo "${HOME}/.local/bin"
}

require_tool curl
require_tool tar

TARGET="$(detect_target)"
VERSION="${TRIGREP_VERSION}"
if [ -z "$VERSION" ]; then
  VERSION="$(resolve_latest_version)"
fi

INSTALL_DIR="$(select_install_dir)"
mkdir -p "$INSTALL_DIR"
if [ ! -w "$INSTALL_DIR" ]; then
  echo "error: install directory is not writable: $INSTALL_DIR" >&2
  echo "set TRIGREP_INSTALL_DIR to a writable directory" >&2
  exit 1
fi

ASSET="trigrep-${VERSION}-${TARGET}.tar.gz"
BASE_URL="https://github.com/${TRIGREP_REPO}/releases/download/${VERSION}"
ASSET_URL="${BASE_URL}/${ASSET}"
CHECKSUMS_URL="${BASE_URL}/checksums.txt"

TMP_DIR="$(mktemp -d "${TMPDIR:-/tmp}/trigrep-install.XXXXXX")"
trap 'rm -rf "$TMP_DIR"' EXIT

ARCHIVE_PATH="${TMP_DIR}/${ASSET}"
CHECKSUMS_PATH="${TMP_DIR}/checksums.txt"

echo "==> Installing trigrep ${VERSION} (${TARGET})"
echo "==> Downloading ${ASSET_URL}"
curl -fsSL --retry 3 -o "$ARCHIVE_PATH" "$ASSET_URL"

echo "==> Downloading ${CHECKSUMS_URL}"
if curl -fsSL --retry 3 -o "$CHECKSUMS_PATH" "$CHECKSUMS_URL"; then
  EXPECTED_SHA="$(awk -v name="$ASSET" '$2 == name { print $1 }' "$CHECKSUMS_PATH")"
  if [ -z "$EXPECTED_SHA" ]; then
    echo "error: checksum for ${ASSET} not found in checksums.txt" >&2
    exit 1
  fi

  ACTUAL_SHA="$(sha256_file "$ARCHIVE_PATH")"
  if [ "$EXPECTED_SHA" != "$ACTUAL_SHA" ]; then
    echo "error: checksum mismatch for ${ASSET}" >&2
    echo "expected: $EXPECTED_SHA" >&2
    echo "actual:   $ACTUAL_SHA" >&2
    exit 1
  fi

  echo "==> Checksum verified"
else
  echo "==> Warning: checksums.txt not found for ${VERSION}; skipping checksum verification" >&2
fi
tar -xzf "$ARCHIVE_PATH" -C "$TMP_DIR"

if [ ! -f "${TMP_DIR}/trigrep" ]; then
  echo "error: trigrep binary not found in archive" >&2
  exit 1
fi

install -m 0755 "${TMP_DIR}/trigrep" "${INSTALL_DIR}/trigrep"

echo "==> Installed to ${INSTALL_DIR}/trigrep"
if "${INSTALL_DIR}/trigrep" --version >/dev/null 2>&1; then
  "${INSTALL_DIR}/trigrep" --version
else
  echo "==> Warning: installed binary does not support --version; use 'trigrep --help'" >&2
  "${INSTALL_DIR}/trigrep" --help >/dev/null
fi

case ":$PATH:" in
  *":${INSTALL_DIR}:"*) ;;
  *)
    echo "==> Add ${INSTALL_DIR} to your PATH if needed"
    ;;
esac
