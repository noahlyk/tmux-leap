#!/usr/bin/env bash
# tmux-leap installer: downloads the latest release binary, verifies its
# checksum and installs it. No root needed.
#
#   curl -fsSL https://raw.githubusercontent.com/noahlyk/tmux-leap/main/install.sh | bash
#
# Environment:
#   TMUX_LEAP_INSTALL_DIR  where to put the binary (default: ~/.local/bin)
#   TMUX_LEAP_VERSION      a release tag such as v1.10.1 (default: latest)
set -euo pipefail

REPO="noahlyk/tmux-leap"
INSTALL_DIR="${TMUX_LEAP_INSTALL_DIR:-$HOME/.local/bin}"
VERSION="${TMUX_LEAP_VERSION:-latest}"

die() {
    echo "install.sh: $*" >&2
    exit 1
}

[ "$(uname -s)" = "Linux" ] || die "release binaries are Linux only. On other systems build from source: cargo install --git https://github.com/$REPO"

case "$(uname -m)" in
    x86_64 | amd64) ARCH="x86_64" ;;
    *) die "no release binary for $(uname -m). Build from source: cargo install --git https://github.com/$REPO" ;;
esac

command -v curl > /dev/null || die "curl is required"
command -v tar > /dev/null || die "tar is required"
command -v sha256sum > /dev/null || die "sha256sum is required"

ASSET="tmux-leap-linux-$ARCH"
if [ "$VERSION" = "latest" ]; then
    BASE_URL="https://github.com/$REPO/releases/latest/download"
else
    BASE_URL="https://github.com/$REPO/releases/download/$VERSION"
fi

WORK_DIR="$(mktemp -d)"
trap 'rm -rf "$WORK_DIR"' EXIT

echo "==> Downloading $ASSET ($VERSION)"
curl -fsSL "$BASE_URL/$ASSET.tar.gz" -o "$WORK_DIR/$ASSET.tar.gz" || die "download failed: $BASE_URL/$ASSET.tar.gz"
curl -fsSL "$BASE_URL/$ASSET.sha256" -o "$WORK_DIR/$ASSET.sha256" || die "download failed: $BASE_URL/$ASSET.sha256"

echo "==> Verifying checksum"
(cd "$WORK_DIR" && sha256sum --check --status "$ASSET.sha256") || die "checksum mismatch, refusing to install"

tar -xzf "$WORK_DIR/$ASSET.tar.gz" -C "$WORK_DIR" tmux-leap
mkdir -p "$INSTALL_DIR"
install -m 755 "$WORK_DIR/tmux-leap" "$INSTALL_DIR/tmux-leap"

echo "==> Installed $("$INSTALL_DIR/tmux-leap" --version) to $INSTALL_DIR/tmux-leap"

# Runtime dependencies and PATH hints.
command -v tmux > /dev/null || echo "!! tmux is not installed; tmux-leap needs it"
command -v fzf > /dev/null || echo "!! fzf is not installed; tmux-leap needs it for the picker"
case ":$PATH:" in
    *":$INSTALL_DIR:"*) ;;
    *) echo "!! $INSTALL_DIR is not on your PATH. Add this to your shell profile:"
       echo "     export PATH=\"$INSTALL_DIR:\$PATH\"" ;;
esac

cat << 'EOF'

==> Next steps
    tmux-leap add ~/Code --depth 1   # add your project folders
    tmux-leap                        # open the picker
    tmux-leap edit                   # settings live in ~/.config/tmux-leap/config.ron

    Bind it in ~/.tmux.conf:
      bind-key -n C-f popup -E -d '#{pane_current_path}' 'tmux-leap'
EOF
