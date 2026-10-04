PREFIX ?= /usr/local
DESTDIR ?=
INSTALL_METHOD ?= manual
CARGO ?= cargo
RUSTC ?= rustc
CARGO_TARGET_DIR ?= target
BINARY_DIR ?= $(CARGO_TARGET_DIR)/release
CARGO_BUILD_FLAGS ?= --release --locked --workspace --bins
DESTDIR_ARGS = $(if $(strip $(DESTDIR)),--destdir "$(DESTDIR)")

BINDIR = $(DESTDIR)$(PREFIX)/bin
LIBEXECDIR = $(DESTDIR)$(PREFIX)/libexec
DATADIR = $(DESTDIR)$(PREFIX)/share
APPDIR = $(DATADIR)/cadiswave
DESKTOPDIR = $(DATADIR)/applications
DOCDIR = $(DATADIR)/doc/cadiswave
LICENSEDIR = $(DATADIR)/licenses/cadiswave
PUBLIC_BINARIES = cadiswave cadiswave-daemon cadiswave-diag cadiswave-probe
BINARIES = $(PUBLIC_BINARIES) cadiswave-maintenance
DOCS = ARCHITECTURE.md hardware-support.md install-bazzite.md protocol.md troubleshooting.md migration.md localization.md upstream.md
ASSETS = cadiswave.desktop cadiswave-autostart.desktop VERSION data/style.css \
	wireplumber/51-cadiswave-wave-xlr.conf pipewire/52-cadiswave-mixes.conf \
	io.github.RamaAditya49.CadisWave.metainfo.xml icons/cadiswave.svg icons/cadiswave-white.svg \
	icons/cadiswave-black.svg icons/cadiswave-red.svg icons/cadiswave-green.svg icons/cadiswave-orange.svg \
	gnome-extension/extension.js gnome-extension/metadata.json README.md README.id.md CONTRIBUTING.md SECURITY.md \
	packaging/asset-attribution.txt LICENSE $(addprefix docs/,$(DOCS))

.PHONY: all build check-toolchain check-install-context check-binaries check-version check-payload install uninstall
all: build

check-toolchain:
	@test "$$(id -u)" != 0 || { echo 'Build as an ordinary user; use install.sh as the login user for a privileged manual installation.' >&2; exit 1; }
	@set -- $$($(RUSTC) --version); test "$$1 $$2" = 'rustc 1.98.1' || { echo 'CadisWave requires Rust 1.98.1.' >&2; exit 1; }

build: check-toolchain
	CARGO_TARGET_DIR="$(CARGO_TARGET_DIR)" $(CARGO) build $(CARGO_BUILD_FLAGS)

# Reject live root manual installation before any build or payload execution.
# Staging and package-manager builds retain their ordinary build-user workflow.
check-install-context:
	@if test "$(INSTALL_METHOD)" = manual && test -z "$(DESTDIR)" && test "$$(id -u)" = 0; then \
		echo 'Do not run sudo make install. Run install.sh as your login user: it stages and checks the payload before authorizing a trusted root-owned install-payload helper. For a user prefix, run make install PREFIX="$$HOME/.local" without sudo.' >&2; \
		exit 1; \
	fi

check-binaries: check-install-context
	@missing=0; for binary in $(BINARIES); do test -x "$(BINARY_DIR)/$$binary" || missing=1; done; \
	if test "$$missing" = 1; then $(MAKE) build; fi
	@set -e; for binary in $(BINARIES); do \
		test -f "$(BINARY_DIR)/$$binary" && test ! -L "$(BINARY_DIR)/$$binary" && test -x "$(BINARY_DIR)/$$binary" || { echo "Missing regular executable: $(BINARY_DIR)/$$binary" >&2; exit 1; }; \
		test "$$(od -An -tx1 -N4 "$(BINARY_DIR)/$$binary" | tr -d ' \n')" = 7f454c46 || { echo "Not a native ELF binary: $$binary" >&2; exit 1; }; \
	done

check-version: check-binaries
	@set -e; version=$$("$(BINARY_DIR)/cadiswave-maintenance" version --file VERSION); \
	for binary in $(BINARIES); do \
		reported=$$("$(BINARY_DIR)/$$binary" --version); \
		test "$$reported" = "$$binary $$version" || { echo "Compiled version mismatch: $$binary (expected $$version, got $$reported)" >&2; exit 1; }; \
	done

check-payload: check-version
	@set -e; for asset in $(ASSETS); do \
		test -f "$$asset" && test ! -L "$$asset" || { echo "Missing regular payload file: $$asset" >&2; exit 1; }; \
	done

install: check-payload
	# This check MUST precede every destination creation or overwrite.
	"$(BINARY_DIR)/cadiswave-maintenance" record-install --check --prefix "$(PREFIX)" $(DESTDIR_ARGS) --method "$(INSTALL_METHOD)"
	install -dm755 "$(BINDIR)" "$(LIBEXECDIR)"
	@set -e; for binary in $(PUBLIC_BINARIES); do install -m755 "$(BINARY_DIR)/$$binary" "$(BINDIR)/$$binary"; done
	install -m755 "$(BINARY_DIR)/cadiswave-maintenance" "$(LIBEXECDIR)/cadiswave-maintenance"
	install -Dm644 cadiswave.desktop "$(DESKTOPDIR)/cadiswave.desktop"
	install -Dm644 cadiswave-autostart.desktop "$(APPDIR)/cadiswave-autostart.desktop"
	install -Dm644 wireplumber/51-cadiswave-wave-xlr.conf "$(APPDIR)/wireplumber/51-cadiswave-wave-xlr.conf"
	install -Dm644 pipewire/52-cadiswave-mixes.conf "$(APPDIR)/pipewire/52-cadiswave-mixes.conf"
	install -Dm644 VERSION "$(APPDIR)/VERSION"
	install -Dm644 data/style.css "$(APPDIR)/style.css"
	install -Dm644 io.github.RamaAditya49.CadisWave.metainfo.xml "$(DATADIR)/metainfo/io.github.RamaAditya49.CadisWave.metainfo.xml"
	install -Dm644 icons/cadiswave.svg "$(DATADIR)/icons/hicolor/scalable/apps/cadiswave.svg"
	install -dm755 "$(DATADIR)/icons/hicolor/scalable/status" "$(APPDIR)/icons"
	install -m644 icons/cadiswave-white.svg icons/cadiswave-black.svg icons/cadiswave-red.svg icons/cadiswave-green.svg icons/cadiswave-orange.svg "$(DATADIR)/icons/hicolor/scalable/status/"
	install -m644 icons/cadiswave.svg icons/cadiswave-white.svg icons/cadiswave-black.svg icons/cadiswave-red.svg icons/cadiswave-green.svg icons/cadiswave-orange.svg "$(APPDIR)/icons/"
	install -dm755 "$(DATADIR)/gnome-shell/extensions/cadiswave-status@cadis.digital"
	install -m644 gnome-extension/metadata.json gnome-extension/extension.js icons/cadiswave.svg icons/cadiswave-green.svg icons/cadiswave-red.svg icons/cadiswave-orange.svg "$(DATADIR)/gnome-shell/extensions/cadiswave-status@cadis.digital/"
	install -Dm644 README.md "$(DOCDIR)/README.md"
	install -Dm644 README.id.md "$(DOCDIR)/README.id.md"
	install -Dm644 CONTRIBUTING.md "$(DOCDIR)/CONTRIBUTING.md"
	install -Dm644 SECURITY.md "$(DOCDIR)/SECURITY.md"
	install -Dm644 icons/cadiswave.svg "$(DOCDIR)/icons/cadiswave.svg"
	@set -e; for doc in $(DOCS); do install -Dm644 "docs/$$doc" "$(DOCDIR)/docs/$$doc"; done
	install -Dm644 packaging/asset-attribution.txt "$(DOCDIR)/asset-attribution.txt"
	install -Dm644 LICENSE "$(LICENSEDIR)/LICENSE"
	"$(BINARY_DIR)/cadiswave-maintenance" record-install --prefix "$(PREFIX)" $(DESTDIR_ARGS) --method "$(INSTALL_METHOD)"
	@if command -v gtk-update-icon-cache >/dev/null 2>&1; then \
		gtk-update-icon-cache --force --ignore-theme-index "$(DATADIR)/icons/hicolor" || \
		printf '%s\n' 'The desktop icon cache could not be refreshed.' >&2; \
	fi

# Explicit application-files-only removal; DESTDIR never addresses live services.
uninstall:
	@helper="$(BINARY_DIR)/cadiswave-maintenance"; \
	if test ! -x "$$helper"; then helper="$(LIBEXECDIR)/cadiswave-maintenance"; fi; \
	test -x "$$helper" || { echo 'A built or installed native maintenance binary is required.' >&2; exit 1; }; \
	"$$helper" files-only --prefix "$(PREFIX)" $(DESTDIR_ARGS) --yes
