#!/usr/bin/env bash
# Universal cross-distro installer for UnScanMe (Debian, Ubuntu, Mint, Fedora, Arch, openSUSE, etc.)
set -euo pipefail

VERSION="0.1.0"
REPO_URL="https://github.com/J0seph-Mart1n/ImgToPdf-Rust"
RELEASE_API="https://api.github.com/repos/J0seph-Mart1n/ImgToPdf-Rust/releases/latest"
BIN_FILE="UnScanMe-Installer-${VERSION}-linux-x86_64"

MODE="user"
ACTION="install"
for arg in "$@"; do
  case "$arg" in
    --system) MODE="system" ;;
    --user) MODE="user" ;;
    --uninstall) ACTION="uninstall" ;;
    -h|--help)
      echo "Usage: $0 [--user | --system | --uninstall]"
      echo "  --user       Install for current user in ~/.local (default, no root required)"
      echo "  --system     Install system-wide in /usr/local (uses sudo)"
      echo "  --uninstall  Remove UnScanMe"
      exit 0
      ;;
  esac
done

if [ "$MODE" = "system" ]; then
  PREFIX="/usr/local"
  LIB_DIR="/usr/local/lib/unscanme"
  SUDO="sudo"
else
  PREFIX="${XDG_DATA_HOME:-$HOME/.local/share}"
  BIN_DIR="$HOME/.local/bin"
  LIB_DIR="$PREFIX/unscanme"
  SUDO=""
fi

if [ "$ACTION" = "uninstall" ]; then
  if [ "$MODE" = "system" ]; then
    $SUDO rm -rf /usr/local/lib/unscanme /usr/local/bin/unscanme /usr/local/bin/unscanme-uninstall
    $SUDO rm -f /usr/local/share/applications/unscanme.desktop /usr/local/share/icons/hicolor/scalable/apps/unscanme.svg
  else
    rm -rf "$LIB_DIR" "$BIN_DIR/unscanme" "$BIN_DIR/unscanme-uninstall"
    rm -f "$PREFIX/applications/unscanme.desktop" "$PREFIX/icons/hicolor/scalable/apps/unscanme.svg"
  fi
  echo "UnScanMe uninstalled."
  exit 0
fi

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]:-$0}")" 2>/dev/null && pwd || pwd)"
if [ -x "$SCRIPT_DIR/$BIN_FILE" ]; then
  exec "$SCRIPT_DIR/$BIN_FILE" "$@"
fi

TMP_DIR="$(mktemp -d)"
trap 'rm -rf "$TMP_DIR"' EXIT

if command -v curl >/dev/null 2>&1; then
  DL_URL="$(curl -fsSL "$RELEASE_API" 2>/dev/null | grep -Eo 'https://[^"]+Installer-[^"]+-linux-[^"]+' | head -n1 || true)"
  if [ -n "$DL_URL" ]; then
    echo "Downloading $(basename "$DL_URL")..."
    curl -fL "$DL_URL" -o "$TMP_DIR/unscanme-installer"
    chmod +x "$TMP_DIR/unscanme-installer"
    exec "$TMP_DIR/unscanme-installer" "$@"
  fi
fi

REPO_ROOT="$(cd "$SCRIPT_DIR/.." 2>/dev/null && pwd || pwd)"
if [ ! -f "$REPO_ROOT/desktop/Cargo.toml" ]; then
  REPO_ROOT="$(cd "$SCRIPT_DIR/../.." 2>/dev/null && pwd || pwd)"
fi
if [ ! -f "$REPO_ROOT/desktop/Cargo.toml" ]; then
  echo "Cloning $REPO_URL..."
  git clone --depth 1 "$REPO_URL" "$TMP_DIR/src"
  REPO_ROOT="$TMP_DIR/src"
fi

export PATH="$HOME/.cargo/bin:$PATH"
if ! command -v cargo >/dev/null 2>&1; then
  echo "Error: Neither a prebuilt release nor Cargo/Rust was found." >&2
  exit 1
fi

cd "$REPO_ROOT/desktop"
if command -v dx >/dev/null 2>&1; then
  dx build --desktop --release
  APP_DIR="$REPO_ROOT/desktop/target/dx/unscanme/release/linux/app"
  BIN_SRC="$APP_DIR/unscanme"
  ASSETS_SRC="$APP_DIR/assets"
else
  cargo build --release --locked
  BIN_SRC="$REPO_ROOT/desktop/target/release/unscanme"
  ASSETS_SRC=""
fi

if [ "$MODE" = "system" ]; then
  BIN_DIR="/usr/local/bin"
  SHARE_DIR="/usr/local/share"
else
  BIN_DIR="$HOME/.local/bin"
  SHARE_DIR="${XDG_DATA_HOME:-$HOME/.local/share}"
fi

$SUDO mkdir -p "$LIB_DIR" "$BIN_DIR" "$SHARE_DIR/applications" "$SHARE_DIR/icons/hicolor/scalable/apps"
$SUDO install -m 755 "$BIN_SRC" "$LIB_DIR/unscanme"
if [ -n "$ASSETS_SRC" ] && [ -d "$ASSETS_SRC" ]; then
  $SUDO rm -rf "$LIB_DIR/assets"
  $SUDO cp -r "$ASSETS_SRC" "$LIB_DIR/assets"
fi
$SUDO ln -sf "$LIB_DIR/unscanme" "$BIN_DIR/unscanme"

if [ -f "$REPO_ROOT/pkgbuild/unscanme.svg" ]; then
  $SUDO install -m 644 "$REPO_ROOT/pkgbuild/unscanme.svg" "$SHARE_DIR/icons/hicolor/scalable/apps/unscanme.svg"
fi
if [ -f "$REPO_ROOT/pkgbuild/unscanme.desktop" ]; then
  sed "s|^Exec=.*|Exec=\"$BIN_DIR/unscanme\" %f|" "$REPO_ROOT/pkgbuild/unscanme.desktop" > "$TMP_DIR/unscanme.desktop"
  $SUDO install -m 644 "$TMP_DIR/unscanme.desktop" "$SHARE_DIR/applications/unscanme.desktop"
fi

echo "UnScanMe installed to $BIN_DIR/unscanme"
