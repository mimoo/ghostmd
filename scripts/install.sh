#!/bin/bash
set -euo pipefail

REPO="mimoo/ghostmd"
INSTALL_DIR="/Applications"
APP_NAME="GhostMD.app"
STATE_DIR="$HOME/.ghostmd"

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
RELEASE_JSON=$(curl -fsSL "https://api.github.com/repos/$REPO/releases/latest" 2>/dev/null) \
  || error "No releases found. Check https://github.com/$REPO/releases"
if command -v jq &>/dev/null; then
  TAG=$(echo "$RELEASE_JSON" | jq -r '.tag_name')
else
  TAG=$(echo "$RELEASE_JSON" | grep '"tag_name"' | sed -E 's/.*"tag_name":\s*"([^"]+)".*/\1/')
fi

[ -n "$TAG" ] && [ "$TAG" != "null" ] || error "Could not determine latest release."
info "Latest release: $TAG"

# check if already up to date
mkdir -p "$STATE_DIR"
CURRENT=$(cat "$STATE_DIR/version" 2>/dev/null || echo "")
if [ "$CURRENT" = "$TAG" ] && [ -d "$INSTALL_DIR/$APP_NAME" ]; then
  success "Already up to date ($TAG)."
  exit 0
fi

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

# save installed version
echo "$TAG" > "$STATE_DIR/version"
rm -f "$STATE_DIR/latest_available"

# create CLI command
read -r -d '' CLI_SCRIPT << 'CLI' || true
#!/bin/bash
REPO="mimoo/ghostmd"
STATE_DIR="$HOME/.ghostmd"

check_update() {
  # only check once a day
  if [ -f "$STATE_DIR/last_check" ]; then
    last=$(cat "$STATE_DIR/last_check")
    now=$(date +%s)
    [ $((now - last)) -lt 86400 ] && show_update_msg && return
  fi

  # run check in background so launch isn't delayed
  (
    mkdir -p "$STATE_DIR"
    date +%s > "$STATE_DIR/last_check"
    latest=$(curl -fsSL --max-time 5 "https://api.github.com/repos/$REPO/releases/latest" 2>/dev/null \
      | grep '"tag_name"' | sed -E 's/.*"tag_name":\s*"([^"]+)".*/\1/')
    current=$(cat "$STATE_DIR/version" 2>/dev/null || echo "")
    if [ -n "$latest" ] && [ "$latest" != "$current" ]; then
      echo "$latest" > "$STATE_DIR/latest_available"
    else
      rm -f "$STATE_DIR/latest_available"
    fi
  ) &>/dev/null &

  show_update_msg
}

show_update_msg() {
  if [ -f "$STATE_DIR/latest_available" ]; then
    latest=$(cat "$STATE_DIR/latest_available")
    printf "\033[0;33mUpdate available: %s → run 'ghostmd update'\033[0m\n" "$latest"
  fi
}

case "${1:-}" in
  update)
    echo "Updating ghostmd..."
    curl -fsSL "https://raw.githubusercontent.com/$REPO/main/scripts/install.sh" | bash
    ;;
  version)
    cat "$STATE_DIR/version" 2>/dev/null || echo "unknown"
    ;;
  *)
    check_update
    open -a /Applications/GhostMD.app "$@"
    ;;
esac
CLI

install_cli() {
  local bin_dir="$1"
  local use_sudo="${2:-false}"

  if [ "$use_sudo" = "true" ]; then
    sudo mkdir -p "$bin_dir"
    echo "$CLI_SCRIPT" | sudo tee "$bin_dir/ghostmd" >/dev/null
    sudo chmod +x "$bin_dir/ghostmd"
  else
    mkdir -p "$bin_dir"
    echo "$CLI_SCRIPT" > "$bin_dir/ghostmd"
    chmod +x "$bin_dir/ghostmd"
  fi
  success "Installed CLI: $bin_dir/ghostmd"
}

if [ -w /usr/local/bin ] || [ -w /usr/local ]; then
  install_cli "/usr/local/bin"
elif sudo -n true 2>/dev/null || [ -t 0 ]; then
  info "Creating CLI command (requires sudo)..."
  install_cli "/usr/local/bin" true
else
  BIN_DIR="$HOME/.local/bin"
  install_cli "$BIN_DIR"
  if ! echo "$PATH" | tr ':' '\n' | grep -qx "$BIN_DIR"; then
    info "Add $BIN_DIR to your PATH to use the 'ghostmd' command."
  fi
fi

echo ""
success "ghostmd installed ($TAG)! Run 'ghostmd' or open from Applications."
