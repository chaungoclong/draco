#!/usr/bin/env sh
set -eu

REPO="chaungoclong/draco"
INSTALL_DIR="${DRACO_INSTALL_DIR:-$HOME/.local/bin}"

detect_platform() {
  OS="$(uname -s)"
  ARCH="$(uname -m)"

  case "$OS" in
    Linux)  OS_PART="linux" ;;
    Darwin) OS_PART="macos" ;;
    *)
      echo "Unsupported OS: $OS. Download manually from https://github.com/$REPO/releases" >&2
      exit 1
      ;;
  esac

  case "$ARCH" in
    x86_64|amd64)  ARCH_PART="x86_64" ;;
    arm64|aarch64) ARCH_PART="aarch64" ;;
    *)
      echo "Unsupported architecture: $ARCH" >&2
      exit 1
      ;;
  esac

  if [ "$OS_PART" = "linux" ] && [ "$ARCH_PART" != "x86_64" ]; then
    echo "Prebuilt Linux binaries currently support x86_64. Please build from source with cargo." >&2
    exit 1
  fi

  ASSET_TARGET="${ARCH_PART}-${OS_PART}"
}

resolve_tag() {
  if [ -n "${DRACO_VERSION:-}" ]; then
    TAG="$DRACO_VERSION"
    return
  fi
  TAG="$(curl -fsSL "https://api.github.com/repos/$REPO/releases/latest" \
    | sed -n 's/.*"tag_name"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p' \
    | head -n 1)"
  if [ -z "$TAG" ]; then
    echo "Could not determine latest release tag from GitHub." >&2
    exit 1
  fi
}

verify_checksum() {
  ARCHIVE_FILE="$1"
  SUMS_FILE="$2"
  ARCHIVE_NAME="$(basename "$ARCHIVE_FILE")"
  EXPECTED="$(grep "  ${ARCHIVE_NAME}$" "$SUMS_FILE" | awk '{print $1}' || true)"
  if [ -z "$EXPECTED" ]; then
    return 0
  fi
  if command -v sha256sum >/dev/null 2>&1; then
    ACTUAL="$(sha256sum "$ARCHIVE_FILE" | awk '{print $1}')"
  elif command -v shasum >/dev/null 2>&1; then
    ACTUAL="$(shasum -a 256 "$ARCHIVE_FILE" | awk '{print $1}')"
  else
    return 0
  fi
  if [ "$EXPECTED" != "$ACTUAL" ]; then
    echo "SHA256 checksum mismatch for $ARCHIVE_NAME!" >&2
    exit 1
  fi
  echo "Checksum verified (SHA256)."
}

main() {
  detect_platform
  resolve_tag

  ARCHIVE="draco-${TAG}-${ASSET_TARGET}.tar.gz"
  BASE_URL="https://github.com/$REPO/releases/download/${TAG}"
  TMP_DIR="$(mktemp -d)"
  trap 'rm -rf "$TMP_DIR"' EXIT

  echo "Downloading Draco ${TAG} (${ASSET_TARGET})..."
  curl -fsSL "${BASE_URL}/${ARCHIVE}" -o "${TMP_DIR}/${ARCHIVE}"
  if curl -fsSL "${BASE_URL}/SHA256SUMS.txt" -o "${TMP_DIR}/SHA256SUMS.txt" 2>/dev/null; then
    verify_checksum "${TMP_DIR}/${ARCHIVE}" "${TMP_DIR}/SHA256SUMS.txt"
  fi

  tar -xzf "${TMP_DIR}/${ARCHIVE}" -C "$TMP_DIR"
  mkdir -p "$INSTALL_DIR"
  EXTRACTED_DIR="${TMP_DIR}/draco-${TAG}-${ASSET_TARGET}"
  cp "${EXTRACTED_DIR}/draco" "${INSTALL_DIR}/draco"
  chmod +x "${INSTALL_DIR}/draco"

  if [ "$OS_PART" = "linux" ] && [ -f "${EXTRACTED_DIR}/draco.desktop" ]; then
    APP_DIR="$HOME/.local/share/applications"
    ICON_DIR="$HOME/.local/share/icons/hicolor/256x256/apps"
    mkdir -p "$APP_DIR" "$ICON_DIR"
    if [ -f "${EXTRACTED_DIR}/draco.png" ]; then
      cp "${EXTRACTED_DIR}/draco.png" "${ICON_DIR}/draco.png"
    fi
    sed "s|^Exec=draco|Exec=${INSTALL_DIR}/draco|" "${EXTRACTED_DIR}/draco.desktop" > "${APP_DIR}/draco.desktop"
  fi

  echo ""
  echo "Draco ${TAG} installed to: ${INSTALL_DIR}/draco"
  case ":$PATH:" in
    *":${INSTALL_DIR}:"*) ;;
    *)
      echo "Add this to your shell profile (~/.bashrc or ~/.zshrc):"
      echo "  export PATH=\"${INSTALL_DIR}:\$PATH\""
      ;;
  esac
  echo "Run: draco ."
}

main "$@"
