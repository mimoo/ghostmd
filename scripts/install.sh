#!/bin/bash
set -euo pipefail

REPO="mimoo/ghostmd"
INSTALL_DIR="/Applications"
APP_NAME="GhostMD.app"

info() { printf "\033[0;34m%s\033[0m\n" "$1"; }
success() { printf "\033[0;32m%s\033[0m\n" "$1"; }
error() { printf "\033[0;31m%s\033[0m\n" "$1" >&2; exit 1; }

# macOS only
[ "$(uname -s)" = "Darwin" ] || error "ghostmd is macOS only."

# detect architecture
case "$(uname -m)" in
  arm64)  TARGET="aarch64-apple-darwin" ;;
  x86_64) TARGET="x86_64-apple-darwin" ;;
  *)      error "Unsupported architecture: $(uname -m)" ;;
esac

info "Detected: $TARGET"

# fetch latest release tag
info "Fetching latest release..."
RELEASE_JSON=$(curl -fsSL "https://api.github.com/repos/$REPO/releases/latest")
if command -v jq &>/dev/null; then
  TAG=$(echo "$RELEASE_JSON" | jq -r '.tag_name')
else
  TAG=$(echo "$RELEASE_JSON" | grep '"tag_name"' | sed -E 's/.*"tag_name":\s*"([^"]+)".*/\1/')
fi

[ -n "$TAG" ] && [ "$TAG" != "null" ] || error "Could not determine latest release."
info "Latest release: $TAG"

# download tarball
TARBALL="GhostMD-${TARGET}.tar.gz"
URL="https://github.com/$REPO/releases/download/$TAG/$TARBALL"
TMPDIR=$(mktemp -d)
trap 'rm -rf "$TMPDIR"' EXIT

info "Downloading $TARBALL..."
curl -fSL --progress-bar -o "$TMPDIR/$TARBALL" "$URL"

# extract
info "Extracting..."
tar -xzf "$TMPDIR/$TARBALL" -C "$TMPDIR"

# remove old version if present
if [ -d "$INSTALL_DIR/$APP_NAME" ]; then
  info "Removing previous installation..."
  rm -rf "$INSTALL_DIR/$APP_NAME"
fi

# install app
info "Installing to $INSTALL_DIR/$APP_NAME..."
mv "$TMPDIR/$APP_NAME" "$INSTALL_DIR/"

# create CLI command
CLI_SCRIPT='#!/bin/bash
open -a /Applications/GhostMD.app "$@"'

if [ -w /usr/local/bin ] || [ -w /usr/local ]; then
  BIN_DIR="/usr/local/bin"
elif sudo -n true 2>/dev/null || [ -t 0 ]; then
  BIN_DIR="/usr/local/bin"
  info "Creating CLI command (requires sudo)..."
  sudo mkdir -p "$BIN_DIR"
  echo "$CLI_SCRIPT" | sudo tee "$BIN_DIR/ghostmd" >/dev/null
  sudo chmod +x "$BIN_DIR/ghostmd"
  success "Installed CLI: $BIN_DIR/ghostmd"
  success "ghostmd installed! Run 'ghostmd' or open from Applications."
  exit 0
else
  BIN_DIR="$HOME/.local/bin"
  mkdir -p "$BIN_DIR"
  if ! echo "$PATH" | tr ':' '\n' | grep -qx "$BIN_DIR"; then
    info "Add $BIN_DIR to your PATH to use the 'ghostmd' command."
  fi
fi

echo "$CLI_SCRIPT" > "$BIN_DIR/ghostmd"
chmod +x "$BIN_DIR/ghostmd"
success "Installed CLI: $BIN_DIR/ghostmd"

echo ""
success "ghostmd installed! Run 'ghostmd' or open from Applications."
