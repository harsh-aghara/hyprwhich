# Maintainer: Harsh Aghara <retrowrench.06@gmail.com>
pkgname=hyprwhich
pkgver=1.0.0
pkgrel=1
pkgdesc="Wayland Layer-Shell keybinding HUD and lookup tool for Hyprland"
arch=('x86_64' 'aarch64')
url="https://github.com/harsh-aghara/hyprwhich"
license=('MIT')
depends=('gtk3' 'gtk-layer-shell')
makedepends=('cargo' 'rust')
optdepends=(
    'rofi-wayland: Rofi launcher integration'
    'wl-clipboard: Clipboard copy support (Ctrl+C)'
    'libnotify: Desktop notifications for copied keybinds'
)
source=("$pkgname-$pkgver.tar.gz::$url/archive/refs/tags/v$pkgver.tar.gz")
sha256sums=('d8cd14836e9a72d035b2a7a4d0d66d838da14824ef837bcf090a9b6a2c40fb29')

prepare() {
    cd "$srcdir/$pkgname-$pkgver"
    export RUSTUP_TOOLCHAIN=stable
}

build() {
    cd "$srcdir/$pkgname-$pkgver"
    cargo build --release --locked
}

check() {
    cd "$srcdir/$pkgname-$pkgver"
    cargo test --release --locked
}

package() {
    cd "$srcdir/$pkgname-$pkgver"
    install -Dm755 "target/release/hyprwhich" "$pkgdir/usr/bin/hyprwhich"
    ln -sf hyprwhich "$pkgdir/usr/bin/hyprwhich-gui"
    install -Dm644 LICENSE "$pkgdir/usr/share/licenses/$pkgname/LICENSE"
    install -Dm644 README.md "$pkgdir/usr/share/doc/$pkgname/README.md"
}
