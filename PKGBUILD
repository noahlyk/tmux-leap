# Maintainer: noahlyk <noahlykins@gmail.com>
pkgname=tmux-leap
pkgver=1.9.4
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

build() {
    cd "$startdir"
    cargo build --release --locked
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
