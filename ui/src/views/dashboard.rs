//! Dashboard view - overview of roles and VMs

use crate::app::{ProxyHopEntry, ProxyVmWizardApp};
use eframe::egui;
use proxy_vm_core::{
    GatewayMode, OpenVpnParsedConfig, ProxyType, VmKind, VmState, WireGuardParsedConfig,
};

pub struct DashboardView;

impl DashboardView {
    pub fn show(app: &mut ProxyVmWizardApp, ui: &mut egui::Ui) {
        // Handle delete confirmation dialog
        if let Some(role) = app.pending_role_delete.clone() {
            egui::Window::new("⚠ Confirm Delete")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ui.ctx(), |ui| {
                    ui.label(format!("Are you sure you want to delete role '{}'?", role));
                    ui.add_space(5.0);
                    ui.label("This will permanently delete:");
                    ui.label(format!("  • Gateway VM: {}-gw", role));
                    ui.label("  • All App VMs for this role".to_string());
                    ui.label(format!("  • Network: {}-inet", role));
                    ui.label("  • All overlay disks".to_string());
                    ui.label("  • Config directory".to_string());
                    ui.add_space(10.0);
                    ui.colored_label(
                        egui::Color32::from_rgb(220, 20, 60),
                        "This action cannot be undone!",
                    );
                    ui.add_space(10.0);
                    ui.horizontal(|ui| {
                        if ui.button("Cancel").clicked() {
                            app.pending_role_delete = None;
                        }
                        if ui.button("🗑 Delete Everything").clicked() {
                            let role_to_delete = role.clone();
                            app.pending_role_delete = None;
                            app.delete_role(&role_to_delete);
                        }
                    });
                });
        }

        // Handle config editor dialog
        if let Some(role) = app.editing_role_config.clone() {
            Self::show_config_editor(app, ui, &role);
        }

        ui.heading("📊 Dashboard");
        ui.add_space(10.0);

        // Quick actions
        ui.horizontal(|ui| {
            if ui.button("➕ Create New Role").clicked() {
                app.start_create_role_wizard();
            }
            ui.separator();
            if let Some(instant) = app.last_refresh {
                let elapsed = instant.elapsed().as_secs();
                ui.label(format!("Last refresh: {}s ago", elapsed));
            }
        });

        ui.add_space(20.0);

        if app.discovered_roles.is_empty() && app.role_vms.is_empty() {
            ui.vertical_centered(|ui| {
                ui.add_space(50.0);
                ui.label("No roles configured yet.");
                ui.add_space(10.0);
                ui.label("Click 'Create New Role' to get started.");
            });
            return;
        }

        // Get all unique roles
        let mut all_roles: Vec<String> = app.discovered_roles.clone();
        for role in app.role_vms.keys() {
            if !all_roles.contains(role) {
                all_roles.push(role.clone());
            }
        }
        all_roles.sort();

        // Role cards
        egui::ScrollArea::vertical().show(ui, |ui| {
            for role in &all_roles {
                Self::show_role_card(app, ui, role);
                ui.add_space(10.0);
            }
        });
    }

    fn show_role_card(app: &mut ProxyVmWizardApp, ui: &mut egui::Ui, role: &str) {
        let vms = app.role_vms.get(role).cloned().unwrap_or_default();
        let gw_vm = vms.iter().find(|v| v.kind == VmKind::ProxyGateway);
        let app_vms: Vec<_> = vms.iter().filter(|v| v.kind == VmKind::App).collect();
        let disp_vms: Vec<_> = vms
            .iter()
            .filter(|v| v.kind == VmKind::DisposableApp)
            .collect();

        egui::Frame::group(ui.style())
            .fill(egui::Color32::from_rgb(30, 35, 45))
            .rounding(8.0)
            .inner_margin(12.0)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.heading(format!("🏷 {}", role));

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui
                            .button("🗑")
                            .on_hover_text("Delete role and all VMs")
                            .clicked()
                        {
                            app.pending_role_delete = Some(role.to_string());
                        }
                        if ui
                            .button("🔧")
                            .on_hover_text("Edit gateway configuration")
                            .clicked()
                        {
                            app.start_editing_role_config(role);
                        }
                    });
                });

                ui.add_space(8.0);

                // Gateway VM section
                ui.horizontal(|ui| {
                    ui.label("Gateway:");
                    if let Some(gw) = gw_vm {
                        let (status_icon, status_color) = match gw.state {
                            VmState::Running => ("🟢", egui::Color32::from_rgb(34, 139, 34)),
                            VmState::Paused => ("🟡", egui::Color32::from_rgb(255, 165, 0)),
                            VmState::ShutOff => ("🔴", egui::Color32::from_rgb(220, 20, 60)),
                            VmState::Unknown => ("⚪", egui::Color32::GRAY),
                        };
                        ui.colored_label(status_color, format!("{} {}", status_icon, gw.name));

                        if gw.state.is_running() {
                            if ui.small_button("⏹ Stop").clicked() {
                                app.stop_vm(&gw.name);
                            }
                        } else if ui.small_button("▶ Start").clicked() {
                            app.start_vm(&gw.name);
                        }
                    } else {
                        ui.label("Not created");
                    }
                });

                // App VMs section
                ui.horizontal(|ui| {
                    ui.label(format!("App VMs: {}", app_vms.len()));
                    if ui.small_button("➕ New App VM").clicked() {
                        app.create_app_vm(role);
                    }
                });

                if !app_vms.is_empty() {
                    ui.indent("app_vms", |ui| {
                        for vm in &app_vms {
                            ui.horizontal(|ui| {
                                let (status_icon, status_color) = match vm.state {
                                    VmState::Running => {
                                        ("🟢", egui::Color32::from_rgb(34, 139, 34))
                                    }
                                    VmState::Paused => ("🟡", egui::Color32::from_rgb(255, 165, 0)),
                                    VmState::ShutOff => {
                                        ("🔴", egui::Color32::from_rgb(220, 20, 60))
                                    }
                                    VmState::Unknown => ("⚪", egui::Color32::GRAY),
                                };
                                ui.colored_label(
                                    status_color,
                                    format!("{} {}", status_icon, vm.name),
                                );

                                if vm.state.is_running() {
                                    if ui.small_button("⏹").on_hover_text("Stop").clicked() {
                                        app.stop_vm(&vm.name);
                                    }
                                } else if ui.small_button("▶").on_hover_text("Start").clicked() {
                                    app.start_vm(&vm.name);
                                }
                            });
                        }
                    });
                }

                // Disposable VMs section
                ui.horizontal(|ui| {
                    ui.label(format!("Disposable: {} active", disp_vms.len()));
                    if ui.small_button("🚀 Launch Disposable").clicked() {
                        app.launch_disposable_vm(role);
                    }
                });

                if !disp_vms.is_empty() {
                    ui.indent("disp_vms", |ui| {
                        for vm in &disp_vms {
                            ui.horizontal(|ui| {
                                ui.colored_label(
                                    egui::Color32::from_rgb(34, 139, 34),
                                    format!("🟢 {}", vm.name),
                                );
                                if ui
                                    .small_button("⏹")
                                    .on_hover_text("Stop (will delete)")
                                    .clicked()
                                {
                                    app.stop_vm(&vm.name);
                                }
                            });
                        }
                    });
                }
            });
    }

    fn show_config_editor(app: &mut ProxyVmWizardApp, ui: &mut egui::Ui, role: &str) {
        egui::Window::new(format!("🔧 Edit Gateway Config: {}", role))
            .collapsible(false)
            .resizable(true)
            .default_width(500.0)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ui.ctx(), |ui| {
                ui.label("Gateway Mode:");
                ui.horizontal(|ui| {
                    ui.radio_value(
                        &mut app.config_editor.gateway_mode,
                        GatewayMode::ProxyChain,
                        "Proxy Chain",
                    );
                    ui.radio_value(
                        &mut app.config_editor.gateway_mode,
                        GatewayMode::WireGuard,
                        "WireGuard",
                    );
                    ui.radio_value(
                        &mut app.config_editor.gateway_mode,
                        GatewayMode::OpenVpn,
                        "OpenVPN",
                    );
                });

                ui.add_space(10.0);
                ui.separator();
                ui.add_space(10.0);

                egui::ScrollArea::vertical()
                    .max_height(400.0)
                    .show(ui, |ui| match app.config_editor.gateway_mode {
                        GatewayMode::ProxyChain => Self::show_proxy_chain_editor(app, ui),
                        GatewayMode::WireGuard => Self::show_wireguard_editor(app, ui),
                        GatewayMode::OpenVpn => Self::show_openvpn_editor(app, ui),
                    });

                ui.add_space(10.0);
                ui.separator();

                if let Some(ref error) = app.config_editor.error {
                    ui.colored_label(egui::Color32::from_rgb(220, 20, 60), error);
                    ui.add_space(5.0);
                }

                ui.checkbox(
                    &mut app.config_editor.restart_after_save,
                    "Restart gateway VM after saving",
                );

                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    if ui.button("Cancel").clicked() {
                        app.cancel_editing_role_config();
                    }
                    if ui.button("💾 Save & Apply").clicked() {
                        app.save_role_config();
                    }
                });
            });
    }

    fn show_proxy_chain_editor(app: &mut ProxyVmWizardApp, ui: &mut egui::Ui) {
        ui.label("Proxy Chain Configuration:");
        ui.add_space(5.0);

        let mut to_remove = None;
        let hop_count = app.config_editor.proxy_hops.len();

        for (i, hop) in app.config_editor.proxy_hops.iter_mut().enumerate() {
            egui::Frame::group(ui.style())
                .inner_margin(6.0)
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(format!("Hop {}", i + 1));
                        if hop_count > 1 {
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    if ui.small_button("✕").clicked() {
                                        to_remove = Some(i);
                                    }
                                },
                            );
                        }
                    });

                    ui.horizontal(|ui| {
                        ui.radio_value(&mut hop.proxy_type, ProxyType::Socks5, "SOCKS5");
                        ui.radio_value(&mut hop.proxy_type, ProxyType::Http, "HTTP");
                    });

                    ui.horizontal(|ui| {
                        ui.label("Host:");
                        ui.add(egui::TextEdit::singleline(&mut hop.host).desired_width(150.0));
                        ui.label("Port:");
                        ui.add(egui::TextEdit::singleline(&mut hop.port).desired_width(60.0));
                    });

                    ui.horizontal(|ui| {
                        ui.label("User:");
                        ui.add(egui::TextEdit::singleline(&mut hop.username).desired_width(100.0));
                        ui.label("Pass:");
                        ui.add(
                            egui::TextEdit::singleline(&mut hop.password)
                                .password(true)
                                .desired_width(100.0),
                        );
                    });
                });
            ui.add_space(3.0);
        }

        if let Some(idx) = to_remove {
            app.config_editor.proxy_hops.remove(idx);
        }

        if app.config_editor.proxy_hops.len() < 8 && ui.button("➕ Add Proxy Hop").clicked() {
            app.config_editor.proxy_hops.push(ProxyHopEntry::default());
        }
    }

    fn show_wireguard_editor(app: &mut ProxyVmWizardApp, ui: &mut egui::Ui) {
        ui.label("WireGuard Configuration:");
        ui.add_space(5.0);

        // List available configs in role directory
        if let Some(role) = &app.editing_role_config.clone() {
            let role_dir = app.global_config.role_dir(role);
            let configs = proxy_vm_core::list_wireguard_configs(&role_dir);

            if !configs.is_empty() {
                ui.label("Available configs in role directory:");
                for (filename, parsed) in &configs {
                    ui.horizontal(|ui| {
                        let is_selected =
                            app.config_editor.wireguard_config.config_filename == *filename;
                        if ui.selectable_label(is_selected, filename).clicked() {
                            app.config_editor.wireguard_config.config_filename = filename.clone();
                        }
                        // Show server info
                        if let Some(peer) = parsed.peers.first() {
                            if let Some(endpoint) = &peer.endpoint {
                                ui.label(
                                    egui::RichText::new(format!("→ {}", endpoint))
                                        .small()
                                        .color(egui::Color32::GRAY),
                                );
                            }
                        }
                    });
                }
                ui.add_space(5.0);
            }
        }

        ui.horizontal(|ui| {
            ui.label("Config file:");
            ui.add(
                egui::TextEdit::singleline(&mut app.config_editor.wireguard_config.config_filename)
                    .desired_width(200.0),
            );
            if ui.button("📂 Import").clicked() {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("WireGuard Config", &["conf"])
                    .pick_file()
                {
                    // Copy file to role directory
                    if let Some(role) = &app.editing_role_config {
                        let role_dir = app.global_config.role_dir(role);
                        if let Some(filename) = path.file_name() {
                            let dest = role_dir.join(filename);
                            if std::fs::copy(&path, &dest).is_ok() {
                                app.config_editor.wireguard_config.config_filename =
                                    filename.to_string_lossy().to_string();
                            }
                        }
                    }
                }
            }
        });

        // Show parsed config info
        if !app
            .config_editor
            .wireguard_config
            .config_filename
            .is_empty()
        {
            if let Some(role) = &app.editing_role_config.clone() {
                let role_dir = app.global_config.role_dir(role);
                let config_path =
                    role_dir.join(&app.config_editor.wireguard_config.config_filename);
                if let Some(parsed) = WireGuardParsedConfig::parse_file(&config_path) {
                    ui.add_space(5.0);
                    egui::Frame::group(ui.style())
                        .fill(egui::Color32::from_rgb(30, 35, 45))
                        .inner_margin(6.0)
                        .show(ui, |ui| {
                            ui.label(egui::RichText::new("Config Details:").small().strong());
                            if let Some(addr) = &parsed.interface_address {
                                ui.label(egui::RichText::new(format!("Address: {}", addr)).small());
                            }
                            for (i, peer) in parsed.peers.iter().enumerate() {
                                ui.label(
                                    egui::RichText::new(format!(
                                        "Peer {}: {}",
                                        i + 1,
                                        peer.name.as_deref().unwrap_or("Unknown")
                                    ))
                                    .small(),
                                );
                                if let Some(endpoint) = &peer.endpoint {
                                    ui.label(
                                        egui::RichText::new(format!("  Endpoint: {}", endpoint))
                                            .small()
                                            .color(egui::Color32::GRAY),
                                    );
                                }
                            }
                        });
                }
            }
        }

        ui.add_space(5.0);
        ui.horizontal(|ui| {
            ui.label("Interface:");
            ui.add(
                egui::TextEdit::singleline(&mut app.config_editor.wireguard_config.interface_name)
                    .hint_text("wg0")
                    .desired_width(100.0),
            );
        });

        ui.checkbox(
            &mut app.config_editor.wireguard_config.route_all_traffic,
            "Route all traffic",
        );
    }

    fn show_openvpn_editor(app: &mut ProxyVmWizardApp, ui: &mut egui::Ui) {
        ui.label("OpenVPN Configuration:");
        ui.add_space(5.0);

        // List available configs in role directory
        if let Some(role) = &app.editing_role_config.clone() {
            let role_dir = app.global_config.role_dir(role);
            let configs = proxy_vm_core::list_openvpn_configs(&role_dir);

            if !configs.is_empty() {
                ui.label("Available configs in role directory:");
                for (filename, parsed) in &configs {
                    ui.horizontal(|ui| {
                        let is_selected =
                            app.config_editor.openvpn_config.config_filename == *filename;
                        if ui.selectable_label(is_selected, filename).clicked() {
                            app.config_editor.openvpn_config.config_filename = filename.clone();
                        }
                        // Show server info
                        if let Some(remote) = parsed.remotes.first() {
                            let info = if let Some(port) = remote.port {
                                format!("→ {}:{}", remote.host, port)
                            } else {
                                format!("→ {}", remote.host)
                            };
                            ui.label(egui::RichText::new(info).small().color(egui::Color32::GRAY));
                        }
                    });
                }
                ui.add_space(5.0);
            }
        }

        ui.horizontal(|ui| {
            ui.label("Config file:");
            ui.add(
                egui::TextEdit::singleline(&mut app.config_editor.openvpn_config.config_filename)
                    .desired_width(200.0),
            );
            if ui.button("📂 Import").clicked() {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("OpenVPN Config", &["ovpn", "conf"])
                    .pick_file()
                {
                    if let Some(role) = &app.editing_role_config {
                        let role_dir = app.global_config.role_dir(role);
                        if let Some(filename) = path.file_name() {
                            let dest = role_dir.join(filename);
                            if std::fs::copy(&path, &dest).is_ok() {
                                app.config_editor.openvpn_config.config_filename =
                                    filename.to_string_lossy().to_string();
                            }
                        }
                    }
                }
            }
        });

        // Show parsed config info
        if !app.config_editor.openvpn_config.config_filename.is_empty() {
            if let Some(role) = &app.editing_role_config.clone() {
                let role_dir = app.global_config.role_dir(role);
                let config_path = role_dir.join(&app.config_editor.openvpn_config.config_filename);
                if let Some(parsed) = OpenVpnParsedConfig::parse_file(&config_path) {
                    ui.add_space(5.0);
                    egui::Frame::group(ui.style())
                        .fill(egui::Color32::from_rgb(30, 35, 45))
                        .inner_margin(6.0)
                        .show(ui, |ui| {
                            ui.label(egui::RichText::new("Config Details:").small().strong());
                            if let Some(proto) = &parsed.protocol {
                                ui.label(
                                    egui::RichText::new(format!("Protocol: {}", proto)).small(),
                                );
                            }
                            ui.label(
                                egui::RichText::new(format!("Servers: {}", parsed.remotes.len()))
                                    .small(),
                            );
                            for (i, remote) in parsed.remotes.iter().take(5).enumerate() {
                                let info = if let Some(port) = remote.port {
                                    format!("  {}. {}:{}", i + 1, remote.host, port)
                                } else {
                                    format!("  {}. {}", i + 1, remote.host)
                                };
                                ui.label(
                                    egui::RichText::new(info).small().color(egui::Color32::GRAY),
                                );
                            }
                            if parsed.remotes.len() > 5 {
                                ui.label(
                                    egui::RichText::new(format!(
                                        "  ... and {} more",
                                        parsed.remotes.len() - 5
                                    ))
                                    .small()
                                    .color(egui::Color32::GRAY),
                                );
                            }
                        });
                }
            }
        }

        ui.add_space(5.0);
        ui.horizontal(|ui| {
            ui.label("Auth file:");
            ui.add(
                egui::TextEdit::singleline(&mut app.config_editor.openvpn_config.auth_filename)
                    .hint_text("(optional)")
                    .desired_width(200.0),
            );
            if ui.button("📂").clicked() {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("Text Files", &["txt"])
                    .pick_file()
                {
                    if let Some(role) = &app.editing_role_config {
                        let role_dir = app.global_config.role_dir(role);
                        if let Some(filename) = path.file_name() {
                            let dest = role_dir.join(filename);
                            if std::fs::copy(&path, &dest).is_ok() {
                                app.config_editor.openvpn_config.auth_filename =
                                    filename.to_string_lossy().to_string();
                            }
                        }
                    }
                }
            }
        });

        ui.checkbox(
            &mut app.config_editor.openvpn_config.route_all_traffic,
            "Route all traffic",
        );
    }
}
