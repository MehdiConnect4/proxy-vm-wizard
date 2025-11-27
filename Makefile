.PHONY: all build release clean install uninstall test lint format appimage deb flatpak

BINARY_NAME = proxy-vm-wizard
VERSION = $(shell grep '^version' Cargo.toml | head -1 | cut -d'"' -f2)

all: build

build:
	cargo build

release:
	cargo build --release
	strip target/release/$(BINARY_NAME)

clean:
	cargo clean
	rm -rf AppDir/
	rm -f *.AppImage *.deb *.tar.gz

test:
	cargo test

lint:
	cargo clippy --all-targets --all-features -- -D warnings

format:
	cargo fmt

format-check:
	cargo fmt -- --check

install: release
	sudo scripts/install.sh

uninstall:
	sudo scripts/uninstall.sh

# Create a tarball for distribution
tarball: release
	mkdir -p dist
	cp target/release/$(BINARY_NAME) dist/
	cp README.md LICENSE dist/
	cp -r assets dist/
	cp scripts/install.sh scripts/uninstall.sh dist/
	cd dist && tar -czvf ../$(BINARY_NAME)-$(VERSION)-linux-x86_64.tar.gz *
	rm -rf dist

# Build .deb package (requires cargo-deb)
deb: release
	cd ui && cargo deb

# Build .rpm package for Fedora (requires rpmbuild)
rpm: release
	mkdir -p ~/rpmbuild/{BUILD,RPMS,SOURCES,SPECS,SRPMS}
	cp packaging/rpm.spec ~/rpmbuild/SPECS/proxy-vm-wizard.spec
	tar -czf ~/rpmbuild/SOURCES/proxy-vm-wizard-$(VERSION).tar.gz \
		--transform 's,^,proxy-vm-wizard-$(VERSION)/,' \
		--exclude='target' --exclude='.git' .
	rpmbuild -ba ~/rpmbuild/SPECS/proxy-vm-wizard.spec

# Build AppImage (requires linuxdeploy)
appimage: release
	mkdir -p AppDir/usr/bin
	mkdir -p AppDir/usr/share/applications
	mkdir -p AppDir/usr/share/icons/hicolor/scalable/apps
	mkdir -p AppDir/usr/share/metainfo
	cp target/release/$(BINARY_NAME) AppDir/usr/bin/
	cp assets/io.github.proxyvmwizard.ProxyVmWizard.desktop AppDir/usr/share/applications/
	cp assets/io.github.proxyvmwizard.ProxyVmWizard.svg AppDir/usr/share/icons/hicolor/scalable/apps/
	cp assets/io.github.proxyvmwizard.ProxyVmWizard.metainfo.xml AppDir/usr/share/metainfo/
	linuxdeploy-x86_64.AppImage --appdir AppDir --output appimage \
		--desktop-file assets/io.github.proxyvmwizard.ProxyVmWizard.desktop \
		--icon-file assets/io.github.proxyvmwizard.ProxyVmWizard.svg

# Generate Flatpak cargo sources
flatpak-sources:
	flatpak-cargo-generator.py Cargo.lock -o flatpak/cargo-sources.json

# Build Flatpak (requires flatpak-builder)
flatpak: flatpak-sources
	flatpak-builder --force-clean build-dir flatpak/io.github.proxyvmwizard.ProxyVmWizard.yml

help:
	@echo "Available targets:"
	@echo "  build      - Build debug version"
	@echo "  release    - Build optimized release version"
	@echo "  clean      - Remove build artifacts"
	@echo "  install    - Install system-wide (requires sudo)"
	@echo "  uninstall  - Remove system installation"
	@echo "  test       - Run tests"
	@echo "  lint       - Run clippy"
	@echo "  format     - Format code with rustfmt"
	@echo "  tarball    - Create distribution tarball"
	@echo "  deb        - Build .deb package (Debian/Ubuntu)"
	@echo "  rpm        - Build .rpm package (Fedora/RHEL)"
	@echo "  appimage   - Build AppImage"
	@echo "  flatpak    - Build Flatpak"


