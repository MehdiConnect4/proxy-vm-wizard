Name:           proxy-vm-wizard
Version:        0.2.9
Release:        1
Summary:        GUI wizard for managing proxy gateway VMs with encryption

License:        MIT
URL:            https://github.com/MehdiConnect4/proxy-vm-wizard

BuildRequires:  rust >= 1.70.0
BuildRequires:  cargo
BuildRequires:  gcc
BuildRequires:  openssl-devel
BuildRequires:  gtk3-devel

Requires:       libvirt-client
Requires:       virt-install
Requires:       qemu-img
Requires:       polkit

%description
Proxy VM Wizard is a local-only, secure Rust GUI for creating
and managing proxy gateway VMs using libvirt/QEMU/KVM.
Features password-based AES-256-GCM encryption for all configuration data.

%build
cargo build --release

%install
install -Dm755 target/release/proxy-vm-wizard %{buildroot}/usr/bin/proxy-vm-wizard
install -Dm644 assets/*.desktop %{buildroot}/usr/share/applications/
install -Dm644 assets/*.svg %{buildroot}/usr/share/icons/hicolor/scalable/apps/
install -Dm644 assets/*.xml %{buildroot}/usr/share/metainfo/

%files
/usr/bin/proxy-vm-wizard
/usr/share/applications/*.desktop
/usr/share/icons/hicolor/scalable/apps/*.svg
/usr/share/metainfo/*.xml


