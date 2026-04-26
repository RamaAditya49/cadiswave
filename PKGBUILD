# Maintainer: Rama Aditya
pkgname=openwave
pkgver=0.1.7
pkgrel=1
pkgdesc="Linux control application for the Elgato Wave XLR"
arch=('any')
url="https://github.com/RamaAditya49/openwave"
license=('MIT')
depends=('python' 'python-gobject' 'gtk4' 'libadwaita' 'libusb' 'pipewire' 'wireplumber' 'alsa-utils')
makedepends=('python-build' 'python-installer' 'python-wheel' 'python-setuptools')
source=("$pkgname-$pkgver.tar.gz::https://github.com/RamaAditya49/openwave/archive/refs/tags/v$pkgver.tar.gz")
sha256sums=('SKIP')

build() {
    cd "$srcdir/$pkgname-$pkgver"
    python -m build --wheel --no-isolation
}

package() {
    cd "$srcdir/$pkgname-$pkgver"

    # Install Python package
    python -m installer --destdir="$pkgdir" dist/*.whl

    # Desktop entry
    install -Dm644 wavexlr.desktop "$pkgdir/usr/share/applications/$pkgname.desktop"

    # License
    install -Dm644 LICENSE "$pkgdir/usr/share/licenses/$pkgname/LICENSE"

    # Docs
    install -Dm644 README.md "$pkgdir/usr/share/doc/$pkgname/README.md"
}
