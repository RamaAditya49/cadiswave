# Fedora 43 native payload; not a universal RHEL/openSUSE binary package.
# Version is supplied by packaging/build-release.sh from the checked VERSION.
Name:           cadiswave
Version:        %{cadiswave_version}
Release:        1%{?dist}
Summary:        Elgato Wave control panel and PipeWire mixing matrix
License:        MIT
URL:            https://github.com/RamaAditya49/cadiswave
ExclusiveArch:  x86_64
Source0:        cadiswave-%{version}.tar.gz
BuildRequires:  gcc
BuildRequires:  gcc-c++
BuildRequires:  clang
BuildRequires:  make
BuildRequires:  pkgconf-pkg-config
BuildRequires:  gtk4-devel >= 4.14
BuildRequires:  libadwaita-devel >= 1.5
BuildRequires:  libusb1-devel
BuildRequires:  desktop-file-utils
BuildRequires:  libappstream-glib
# The release builder installs the checksummed standalone Rust 1.98.1 on PATH.
# Do not select distro cargo macros, which can bypass that exact toolchain.
Requires:       gtk4 >= 4.14
Requires:       libadwaita >= 1.5
Requires:       adwaita-icon-theme
Requires:       polkit
Requires:       libusb1
Requires:       pipewire
Requires:       pipewire-utils
Requires:       wireplumber
Requires:       alsa-utils
Requires:       pulseaudio-utils
Requires:       ladspa-swh-plugins

%description
Control Elgato Wave audio hardware and route application and device sources
through independent PipeWire mixes and output devices. This native package
is built for Fedora 43 on its matching architecture.

%prep
%autosetup

%build
export CARGO_NET_OFFLINE=true
export CARGO_TARGET_DIR=target
make build CARGO_BUILD_FLAGS='--release --frozen --offline --workspace --bins'

%install
make install DESTDIR="%{buildroot}" PREFIX=/usr INSTALL_METHOD=rpm \
    BINARY_DIR=target/release \
    CARGO_BUILD_FLAGS='--release --frozen --offline --workspace --bins'

%check
desktop-file-validate %{buildroot}/usr/share/applications/cadiswave.desktop
appstream-util validate-relax --nonet %{buildroot}/usr/share/metainfo/io.github.RamaAditya49.CadisWave.metainfo.xml

%files
/usr/bin/cadiswave
/usr/bin/cadiswave-daemon
/usr/bin/cadiswave-diag
/usr/bin/cadiswave-probe
/usr/libexec/cadiswave-maintenance
/usr/share/cadiswave/
/usr/share/applications/cadiswave.desktop
/usr/share/metainfo/io.github.RamaAditya49.CadisWave.metainfo.xml
/usr/share/icons/hicolor/scalable/apps/cadiswave.svg
/usr/share/icons/hicolor/scalable/status/cadiswave-white.svg
/usr/share/icons/hicolor/scalable/status/cadiswave-black.svg
/usr/share/icons/hicolor/scalable/status/cadiswave-red.svg
%doc /usr/share/doc/cadiswave/
%license /usr/share/licenses/cadiswave/

%changelog
* Wed Sep 09 2026 CadisWave contributors - 1.0.0-1
- Replace the Python runtime with the pinned Rust workspace and native helpers.
- Preserve saved state and ownership-checked installation and removal.
- Handle absent systemd units and optional HOME defaults; refresh capture-service status after setup.
