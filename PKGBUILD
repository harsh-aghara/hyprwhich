# Maintainer: Hyprland Community <hyprwhich@example.com>
pkgname=hyprwhich
pkgver=1.0.0
pkgrel=1
pkgdesc="Wayland Layer-Shell keybinding HUD and lookup tool for Hyprland"
arch=('x86_64' 'aarch64')
url="https://github.com/hyprwhich/hyprwhich"
license=('MIT')
depends=('gtk3' 'gtk-layer-shell')
makedepends=('cargo' 'rust')
optdepends=(
    'rofi-wayland: Rofi launcher integration'
    'wl-clipboard: Clipboard copy support (Ctrl+C)'
    'libnotify: Desktop notifications for copied keybinds'
)
source=("$pkgname-$pkgver.tar.gz::$url/archive/v$pkgver.tar.gz")
sha256sums=('SKIP')

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
    install -Dm644 README.md "$pkgdir/usr/share/doc/$pkgname/README.md"
}
