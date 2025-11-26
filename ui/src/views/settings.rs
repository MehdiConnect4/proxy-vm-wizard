//! Settings view - global configuration

use crate::app::{ProxyVmWizardApp, StatusLevel};
use eframe::egui;

pub struct SettingsView;

impl SettingsView {
    pub fn show(app: &mut ProxyVmWizardApp, ui: &mut egui::Ui) {
        ui.heading("⚙ Settings");
        ui.add_space(10.0);

        egui::ScrollArea::vertical().show(ui, |ui| {
            // Paths section
            egui::CollapsingHeader::new("📁 Paths")
                .default_open(true)
                .show(ui, |ui| {
                    egui::Grid::new("paths_grid")
                        .num_columns(2)
                        .spacing([10.0, 8.0])
                        .show(ui, |ui| {
                            ui.label("Config Root:");
                            ui.horizontal(|ui| {
                                ui.add(
                                    egui::TextEdit::singleline(&mut app.settings_view.cfg_root)
                                        .desired_width(400.0),
                                );
                                if ui.button("Browse...").clicked() {
                                    if let Some(path) = rfd::FileDialog::new().pick_folder() {
                                        app.settings_view.cfg_root = path.display().to_string();
                                    }
                                }
                            });
                            ui.end_row();

                            ui.label("Images Directory:");
                            ui.horizontal(|ui| {
                                ui.add(
                                    egui::TextEdit::singleline(&mut app.settings_view.images_dir)
                                        .desired_width(400.0),
                                );
                                if ui.button("Browse...").clicked() {
                                    if let Some(path) = rfd::FileDialog::new().pick_folder() {
                                        app.settings_view.images_dir = path.display().to_string();
                                    }
                                }
                            });
                            ui.end_row();
                        });

                    ui.add_space(5.0);
                    ui.label(
                        egui::RichText::new(
                            "Config Root: Where per-role configuration directories are stored.\n\
                         Images Directory: Where qcow2 overlay disks are created.",
                        )
                        .color(egui::Color32::GRAY)
                        .small(),
                    );
                });

            ui.add_space(10.0);

            // Libvirt section
            egui::CollapsingHeader::new("🖥 Libvirt")
                .default_open(true)
                .show(ui, |ui| {
                    egui::Grid::new("libvirt_grid")
                        .num_columns(2)
                        .spacing([10.0, 8.0])
                        .show(ui, |ui| {
                            ui.label("LAN Network:");
                            ui.add(
                                egui::TextEdit::singleline(&mut app.settings_view.lan_net)
                                    .hint_text("lan-net")
                                    .desired_width(200.0),
                            );
                            ui.end_row();
                        });

                    ui.add_space(5.0);
                    ui.label(
                        egui::RichText::new(
                            "LAN Network: The libvirt network your pfSense/gateway connects to.\n\
                         This network must already exist in libvirt.",
                        )
                        .color(egui::Color32::GRAY)
                        .small(),
                    );

                    ui.add_space(10.0);
                    if ui.button("🔌 Test Libvirt Connectivity").clicked() {
                        match app.libvirt.check_libvirt_access() {
                            Ok(_) => {
                                app.set_status(
                                    StatusLevel::Success,
                                    "Libvirt connection successful",
                                );
                            }
                            Err(e) => {
                                app.set_status(StatusLevel::Error, format!("Libvirt error: {}", e));
                            }
                        }
                    }
                });

            ui.add_space(10.0);

            // Defaults section
            egui::CollapsingHeader::new("💾 Defaults")
                .default_open(true)
                .show(ui, |ui| {
                    egui::Grid::new("defaults_grid")
                        .num_columns(2)
                        .spacing([10.0, 8.0])
                        .show(ui, |ui| {
                            ui.label("Gateway RAM (MB):");
                            ui.add(
                                egui::TextEdit::singleline(&mut app.settings_view.gateway_ram)
                                    .desired_width(80.0),
                            );
                            ui.end_row();

                            ui.label("App VM RAM (MB):");
                            ui.add(
                                egui::TextEdit::singleline(&mut app.settings_view.app_ram)
                                    .desired_width(80.0),
                            );
                            ui.end_row();

                            ui.label("Disposable RAM (MB):");
                            ui.add(
                                egui::TextEdit::singleline(&mut app.settings_view.disp_ram)
                                    .desired_width(80.0),
                            );
                            ui.end_row();
                        });
                });

            ui.add_space(10.0);

            // OS Variants section
            egui::CollapsingHeader::new("🐧 OS Variants")
                .default_open(true)
                .show(ui, |ui| {
                    egui::Grid::new("os_grid")
                        .num_columns(2)
                        .spacing([10.0, 8.0])
                        .show(ui, |ui| {
                            ui.label("Debian OS Variant:");
                            egui::ComboBox::from_id_salt("debian_variant")
                                .selected_text(&app.settings_view.debian_variant)
                                .show_ui(ui, |ui| {
                                    for v in ["debian12", "debian13", "debian11"] {
                                        ui.selectable_value(
                                            &mut app.settings_view.debian_variant,
                                            v.to_string(),
                                            v,
                                        );
                                    }
                                });
                            ui.end_row();

                            ui.label("Fedora OS Variant:");
                            egui::ComboBox::from_id_salt("fedora_variant")
                                .selected_text(&app.settings_view.fedora_variant)
                                .show_ui(ui, |ui| {
                                    for v in ["fedora40", "fedora41", "fedora-rawhide", "fedora39"]
                                    {
                                        ui.selectable_value(
                                            &mut app.settings_view.fedora_variant,
                                            v.to_string(),
                                            v,
                                        );
                                    }
                                });
                            ui.end_row();
                        });

                    ui.add_space(5.0);
                    ui.label(
                        egui::RichText::new(
                            "OS Variants are used by virt-install to optimize VM configuration.\n\
                         Run 'osinfo-query os' to see all available variants.",
                        )
                        .color(egui::Color32::GRAY)
                        .small(),
                    );
                });

            ui.add_space(20.0);

            // Error display
            if let Some(ref error) = app.settings_view.error {
                ui.colored_label(egui::Color32::from_rgb(220, 20, 60), error);
                ui.add_space(10.0);
            }

            // Save button
            ui.horizontal(|ui| {
                if ui.button("💾 Save Settings").clicked() {
                    app.settings_view.saved = false;
                    app.save_settings();
                }

                if app.settings_view.saved {
                    ui.colored_label(egui::Color32::from_rgb(34, 139, 34), "✓ Saved");
                }
            });

            ui.add_space(20.0);

            // Info section
            egui::CollapsingHeader::new("ℹ About").show(ui, |ui| {
                ui.label("Proxy VM Wizard");
                ui.label(format!("Version: {}", env!("CARGO_PKG_VERSION")));
                ui.add_space(5.0);
                ui.label("A secure, local-only VM management tool for libvirt/QEMU.");
                ui.label("No network calls, no telemetry, no external services.");
                ui.add_space(10.0);
                ui.label(egui::RichText::new("Recommended Guest OS:").strong());
                ui.label("• Proxy/Gateway VMs: Debian stable (minimal, hardened)");
                ui.label("• App VMs: Debian or Fedora");
            });
        });
    }
}
