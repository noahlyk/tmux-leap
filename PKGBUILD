# Maintainer: noahlyk <noahlykins@gmail.com>
pkgname=tmux-leap
pkgver=1.11.0
pkgrel=1
pkgdesc="tmux leaper, fzf through a list of projects or directories, autosessionizing, history"
arch=('x86_64' 'aarch64')
url="https://github.com/noahlyk/tmux-leap"
license=('MIT')
depends=('fzf' 'tmux')
makedepends=('rust' 'cargo')
options=('!debug')
install=$pkgname.install

source=()
sha256sums=()

# Local builds from a git checkout report the tag, commit count and hash, the same as `make`:
#   1.10.1+2.gbbd181e, or 1.10.1+2.gbbd181e.dirty with uncommitted changes.
# Anywhere without git (or on the release tag) the version above is used.
pkgver() {
    cd "$startdir"
    git rev-parse --git-dir >/dev/null 2>&1 || { echo "$pkgver"; return; }

    local version meta=() described dirty
    if described=$(git describe --tags --long --match 'v[0-9]*' 2>/dev/null); then
        local rest="${described%-g*}"
        version="${rest%-*}"
        version="${version#v}"
        local count="${rest##*-}"
        (( count > 0 )) && meta+=("$count.g${described##*-g}")
    else
        version="0.0.0"
        meta+=("g$(git rev-parse --short HEAD)")
    fi
    [ -n "$(git status --porcelain --untracked-files=no)" ] && meta+=(dirty)

    if (( ${#meta[@]} )); then
        local IFS=.
        version+="+${meta[*]}"
    fi
    echo "$version"
}

build() {
    cd "$startdir"
    TMUX_LEAP_VERSION="$pkgver" cargo build --release --locked
}

package() {
    cd "$startdir"

    # Install compiled binary
    install -Dm755 "target/release/$pkgname" "$pkgdir/usr/bin/$pkgname"

    # Install license
    install -Dm644 "LICENSE" "$pkgdir/usr/share/licenses/$pkgname/LICENSE"

    # Generate and install shell completions for main binary
    install -dm755 "$pkgdir/usr/share/bash-completion/completions"
    install -dm755 "$pkgdir/usr/share/zsh/site-functions"
    install -dm755 "$pkgdir/usr/share/fish/vendor_completions.d"

    "$pkgdir/usr/bin/$pkgname" completion bash > "$pkgdir/usr/share/bash-completion/completions/$pkgname"
    "$pkgdir/usr/bin/$pkgname" completion zsh > "$pkgdir/usr/share/zsh/site-functions/_$pkgname"
    "$pkgdir/usr/bin/$pkgname" completion fish > "$pkgdir/usr/share/fish/vendor_completions.d/$pkgname.fish"
}
