//! Wizard view - create/edit roles

use crate::app::{ProxyHopEntry, ProxyVmWizardApp, WizardMode, WizardStep};
use crate::views::View;
use eframe::egui;
use proxy_vm_core::{GatewayMode, ProxyType};

pub struct WizardView;

impl WizardView {
    pub fn show(app: &mut ProxyVmWizardApp, ui: &mut egui::Ui) {
        let mode_text = match app.wizard.mode {
            WizardMode::Create => "Create New Role",
            WizardMode::Edit => "Edit Role",
        };
        ui.heading(format!("🧙 {} Wizard", mode_text));
        ui.add_space(10.0);

        // Step indicator
        ui.horizontal(|ui| {
            let steps = [
                "1. Role Basics",
                "2. Gateway Config",
                "3. Confirm",
                "4. Execute",
            ];
            for (i, step) in steps.iter().enumerate() {
                let current = match app.wizard.step {
                    WizardStep::RoleBasics => 0,
                    WizardStep::GatewayConfig => 1,
                    WizardStep::Confirmation => 2,
                    WizardStep::Execution => 3,
                };

                if i == current {
                    ui.strong(*step);
                } else if i < current {
                    ui.label(
                        egui::RichText::new(*step).color(egui::Color32::from_rgb(34, 139, 34)),
                    );
                } else {
                    ui.label(egui::RichText::new(*step).color(egui::Color32::GRAY));
                }

                if i < steps.len() - 1 {
                    ui.label("→");
                }
            }
        });

        ui.separator();
        ui.add_space(10.0);

        egui::ScrollArea::vertical().show(ui, |ui| match app.wizard.step {
            WizardStep::RoleBasics => Self::show_step_role_basics(app, ui),
            WizardStep::GatewayConfig => Self::show_step_gateway_config(app, ui),
            WizardStep::Confirmation => Self::show_step_confirmation(app, ui),
            WizardStep::Execution => Self::show_step_execution(app, ui),
        });

        ui.add_space(20.0);
        ui.separator();

        // Navigation buttons
        ui.horizontal(|ui| {
            match app.wizard.step {
                WizardStep::Execution => {
                    // During/after execution, show appropriate buttons
                    if app.wizard.execution_error.is_some() {
                        // Error occurred - offer to clean up and go back or retry
                        if ui.button("🗑 Clean Up & Cancel").clicked() {
                            app.cleanup_wizard_resources();
                            app.reset_wizard();
                            app.navigate_to(View::Dashboard);
                        }
                        if ui.button("← Back to Edit").clicked() {
                            app.cleanup_wizard_resources();
                            app.wizard.step = WizardStep::Confirmation;
                            app.wizard.execution_error = None;
                            app.wizard.execution_messages.clear();
                        }
                    } else if app.wizard.is_executing {
                        // Currently executing - can cancel
                        ui.label("Creating resources...");
                    } else {
                        // Success
                        if ui.button("Done").clicked() {
                            app.reset_wizard();
                            app.navigate_to(View::Dashboard);
                        }
                    }
                }
                _ => {
                    // Normal wizard steps
                    if ui.button("Cancel").clicked() {
                        app.cleanup_wizard_resources();
                        app.reset_wizard();
                        app.navigate_to(View::Dashboard);
                    }
                }
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                match app.wizard.step {
                    WizardStep::RoleBasics => {
                        if ui.button("Next →").clicked() {
                            app.wizard_next_step();
                        }
                    }
                    WizardStep::GatewayConfig => {
                        if ui.button("Next →").clicked() {
                            app.wizard_next_step();
                        }
                        if ui.button("← Back").clicked() {
                            app.wizard_prev_step();
                        }
                    }
                    WizardStep::Confirmation => {
                        if ui.button("Create Role").clicked() {
                            app.wizard_next_step();
                        }
                        if ui.button("← Back").clicked() {
                            app.wizard_prev_step();
                        }
                    }
                    WizardStep::Execution => {
                        // Buttons handled above
                    }
                }
            });
        });
    }

    fn show_step_role_basics(app: &mut ProxyVmWizardApp, ui: &mut egui::Ui) {
        ui.heading("Step 1: Role Basics");
        ui.add_space(10.0);

        egui::Grid::new("role_basics_grid")
            .num_columns(2)
            .spacing([20.0, 10.0])
            .show(ui, |ui| {
                ui.label("Role Name:");
                let response = ui.add(
                    egui::TextEdit::singleline(&mut app.wizard.role_name)
                        .hint_text("e.g., work, bank, personal")
                        .desired_width(200.0),
                );
                if response.changed() {
                    app.wizard.role_name_error = None;
                }
                ui.end_row();

                if let Some(ref error) = app.wizard.role_name_error {
                    ui.label("");
                    ui.colored_label(egui::Color32::from_rgb(220, 20, 60), error);
                    ui.end_row();
                }

                // Gateway template selection
                ui.label("Gateway Template:");
                let gw_templates = app.template_registry.get_gateway_templates();
                if gw_templates.is_empty() {
                    ui.colored_label(
                        egui::Color32::from_rgb(255, 165, 0),
                        "No gateway templates. Add one in Templates view.",
                    );
                } else {
                    let current_label = app
                        .wizard
                        .selected_gw_template_id
                        .as_ref()
                        .and_then(|id| app.template_registry.get(id))
                        .map(|t| t.label.clone())
                        .unwrap_or_else(|| "Select...".to_string());

                    egui::ComboBox::from_id_salt("gw_template")
                        .selected_text(&current_label)
                        .show_ui(ui, |ui| {
                            for template in gw_templates {
                                let is_selected = app.wizard.selected_gw_template_id.as_ref()
                                    == Some(&template.id);
                                if ui.selectable_label(is_selected, &template.label).clicked() {
                                    app.wizard.selected_gw_template_id = Some(template.id.clone());
                                }
                            }
                        });
                }
                ui.end_row();

                // App template selection
                ui.label("App Template:");
                let app_templates = app.template_registry.get_app_templates();
                if app_templates.is_empty() {
                    ui.colored_label(
                        egui::Color32::from_rgb(255, 165, 0),
                        "No app templates. Add one in Templates view.",
                    );
                } else {
                    let current_label = app
                        .wizard
                        .selected_app_template_id
                        .as_ref()
                        .and_then(|id| app.template_registry.get(id))
                        .map(|t| t.label.clone())
                        .unwrap_or_else(|| "Select...".to_string());

                    egui::ComboBox::from_id_salt("app_template")
                        .selected_text(&current_label)
                        .show_ui(ui, |ui| {
                            for template in app_templates {
                                let is_selected = app.wizard.selected_app_template_id.as_ref()
                                    == Some(&template.id);
                                if ui.selectable_label(is_selected, &template.label).clicked() {
                                    app.wizard.selected_app_template_id = Some(template.id.clone());
                                    // Default disp to same as app
                                    if app.wizard.selected_disp_template_id.is_none() {
                                        app.wizard.selected_disp_template_id =
                                            Some(template.id.clone());
                                    }
                                }
                            }
                        });
                }
                ui.end_row();

                // Disposable template selection
                ui.label("Disposable Template:");
                let disp_templates = app.template_registry.get_app_templates();
                if disp_templates.is_empty() {
                    ui.label("(Same as App template)");
                } else {
                    let current_label = app
                        .wizard
                        .selected_disp_template_id
                        .as_ref()
                        .and_then(|id| app.template_registry.get(id))
                        .map(|t| t.label.clone())
                        .unwrap_or_else(|| "Select...".to_string());

                    egui::ComboBox::from_id_salt("disp_template")
                        .selected_text(&current_label)
                        .show_ui(ui, |ui| {
                            for template in disp_templates {
                                let is_selected = app.wizard.selected_disp_template_id.as_ref()
                                    == Some(&template.id);
                                if ui.selectable_label(is_selected, &template.label).clicked() {
                                    app.wizard.selected_disp_template_id =
                                        Some(template.id.clone());
                                }
                            }
                        });
                }
                ui.end_row();
            });

        // Show computed names
        if !app.wizard.role_name.is_empty() {
            ui.add_space(20.0);
            ui.label("Computed resource names:");
            let role = proxy_vm_core::normalize_role_name(&app.wizard.role_name);
            ui.code(format!("Gateway VM: {}-gw", role));
            ui.code(format!("Internal network: {}-inet", role));
            ui.code(format!(
                "Config directory: {}/{}",
                app.global_config.cfg.root.display(),
                role
            ));
        }
    }

    fn show_step_gateway_config(app: &mut ProxyVmWizardApp, ui: &mut egui::Ui) {
        // Show mode change confirmation dialog if pending
        if let Some(new_mode) = app.wizard.pending_mode_change {
            egui::Window::new("⚠ Change Protocol?")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ui.ctx(), |ui| {
                    ui.label(format!(
                        "You have data entered for {}.",
                        app.wizard.gateway_mode.display_name()
                    ));
                    ui.label(format!(
                        "Switching to {} will clear that data.",
                        new_mode.display_name()
                    ));
                    ui.add_space(10.0);
                    ui.label("You can only use one protocol at a time.");
                    ui.add_space(10.0);
                    ui.horizontal(|ui| {
                        if ui
                            .button(format!("Keep {}", app.wizard.gateway_mode.display_name()))
                            .clicked()
                        {
                            app.cancel_mode_change();
                        }
                        if ui
                            .button(format!("Switch to {}", new_mode.display_name()))
                            .clicked()
                        {
                            app.confirm_mode_change();
                        }
                    });
                });
        }

        ui.heading("Step 2: Gateway Configuration");
        ui.add_space(10.0);

        ui.label("Gateway Mode (choose one):");
        ui.horizontal(|ui| {
            let current_mode = app.wizard.gateway_mode;
            if ui
                .radio(current_mode == GatewayMode::ProxyChain, "Proxy Chain")
                .clicked()
            {
                app.request_mode_change(GatewayMode::ProxyChain);
            }
            if ui
                .radio(current_mode == GatewayMode::WireGuard, "WireGuard")
                .clicked()
            {
                app.request_mode_change(GatewayMode::WireGuard);
            }
            if ui
                .radio(current_mode == GatewayMode::OpenVpn, "OpenVPN")
                .clicked()
            {
                app.request_mode_change(GatewayMode::OpenVpn);
            }
        });

        ui.add_space(20.0);

        match app.wizard.gateway_mode {
            GatewayMode::ProxyChain => Self::show_proxy_chain_config(app, ui),
            GatewayMode::WireGuard => Self::show_wireguard_config(app, ui),
            GatewayMode::OpenVpn => Self::show_openvpn_config(app, ui),
        }
    }

    fn show_proxy_chain_config(app: &mut ProxyVmWizardApp, ui: &mut egui::Ui) {
        ui.label("Configure proxy chain (1-8 hops):");
        ui.add_space(10.0);

        let mut to_remove = None;
        let hop_count = app.wizard.proxy_hops.len();

        for (i, hop) in app.wizard.proxy_hops.iter_mut().enumerate() {
            egui::Frame::group(ui.style())
                .inner_margin(8.0)
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(format!("Hop {}", i + 1));

                        if hop_count > 1 {
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    if ui.small_button("✕ Remove").clicked() {
                                        to_remove = Some(i);
                                    }
                                },
                            );
                        }
                    });

                    egui::Grid::new(format!("hop_grid_{}", i))
                        .num_columns(2)
                        .spacing([10.0, 6.0])
                        .show(ui, |ui| {
                            ui.label("Type:");
                            ui.horizontal(|ui| {
                                ui.radio_value(&mut hop.proxy_type, ProxyType::Socks5, "SOCKS5");
                                ui.radio_value(&mut hop.proxy_type, ProxyType::Http, "HTTP");
                            });
                            ui.end_row();

                            ui.label("Host:");
                            ui.add(
                                egui::TextEdit::singleline(&mut hop.host)
                                    .hint_text("IP or hostname")
                                    .desired_width(200.0),
                            );
                            ui.end_row();

                            ui.label("Port:");
                            ui.add(
                                egui::TextEdit::singleline(&mut hop.port)
                                    .hint_text("1080")
                                    .desired_width(80.0),
                            );
                            ui.end_row();

                            ui.label("Username:");
                            ui.add(
                                egui::TextEdit::singleline(&mut hop.username)
                                    .hint_text("(optional)")
                                    .desired_width(150.0),
                            );
                            ui.end_row();

                            ui.label("Password:");
                            ui.add(
                                egui::TextEdit::singleline(&mut hop.password)
                                    .password(true)
                                    .hint_text("(optional)")
                                    .desired_width(150.0),
                            );
                            ui.end_row();

                            ui.label("Label:");
                            ui.add(
                                egui::TextEdit::singleline(&mut hop.label)
                                    .hint_text("(optional, e.g., 'US Exit')")
                                    .desired_width(150.0),
                            );
                            ui.end_row();
                        });

                    // Test status display
                    ui.horizontal(|ui| {
                        if let Some(status) = hop.test_status {
                            if status {
                                ui.colored_label(
                                    egui::Color32::from_rgb(34, 139, 34),
                                    "✓ Connected",
                                );
                            } else {
                                ui.colored_label(
                                    egui::Color32::from_rgb(220, 20, 60),
                                    format!(
                                        "✗ {}",
                                        hop.test_message.as_deref().unwrap_or("Failed")
                                    ),
                                );
                            }
                        }
                    });
                });
            ui.add_space(5.0);
        }

        // Handle removal
        if let Some(idx) = to_remove {
            app.wizard.proxy_hops.remove(idx);
        }

        // Add hop button
        if app.wizard.proxy_hops.len() < 8 && ui.button("➕ Add Proxy Hop").clicked() {
            app.wizard.proxy_hops.push(ProxyHopEntry::default());
        }

        // Handle test button clicks (separate to avoid borrow issues)
        ui.add_space(10.0);
        ui.horizontal(|ui| {
            for i in 0..app.wizard.proxy_hops.len() {
                if ui.small_button(format!("Test Hop {}", i + 1)).clicked() {
                    app.test_proxy_connection(i);
                }
            }
        });
    }

    fn show_wireguard_config(app: &mut ProxyVmWizardApp, ui: &mut egui::Ui) {
        ui.label("WireGuard Configuration:");
        ui.label(egui::RichText::new(
            "Select your WireGuard config file. It will be copied to the role directory and accessible as /proxy/<filename> inside the VM."
        ).color(egui::Color32::GRAY).small());
        ui.add_space(10.0);

        egui::Grid::new("wg_grid")
            .num_columns(2)
            .spacing([10.0, 8.0])
            .show(ui, |ui| {
                ui.label("Config file:");
                ui.horizontal(|ui| {
                    ui.add(
                        egui::TextEdit::singleline(
                            &mut app.wizard.wireguard_config.config_filename,
                        )
                        .hint_text("Click Browse to select...")
                        .desired_width(250.0),
                    );
                    if ui.button("📂 Browse...").clicked() {
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter("WireGuard Config", &["conf"])
                            .add_filter("All Files", &["*"])
                            .pick_file()
                        {
                            // Store full path temporarily, we'll copy it during execution
                            app.wizard.wireguard_config.config_filename =
                                path.display().to_string();
                        }
                    }
                });
                ui.end_row();

                ui.label("Interface name:");
                ui.add(
                    egui::TextEdit::singleline(&mut app.wizard.wireguard_config.interface_name)
                        .hint_text("wg0")
                        .desired_width(100.0),
                );
                ui.end_row();

                ui.label("Route all traffic:");
                ui.checkbox(&mut app.wizard.wireguard_config.route_all_traffic, "");
                ui.end_row();
            });
    }

    fn show_openvpn_config(app: &mut ProxyVmWizardApp, ui: &mut egui::Ui) {
        ui.label("OpenVPN Configuration:");
        ui.label(egui::RichText::new(
            "Select your OpenVPN config files. They will be copied to the role directory and accessible as /proxy/<filename> inside the VM."
        ).color(egui::Color32::GRAY).small());
        ui.add_space(10.0);

        egui::Grid::new("ovpn_grid")
            .num_columns(2)
            .spacing([10.0, 8.0])
            .show(ui, |ui| {
                ui.label("Config file:");
                ui.horizontal(|ui| {
                    ui.add(
                        egui::TextEdit::singleline(&mut app.wizard.openvpn_config.config_filename)
                            .hint_text("Click Browse to select...")
                            .desired_width(250.0),
                    );
                    if ui.button("📂 Browse...").clicked() {
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter("OpenVPN Config", &["ovpn", "conf"])
                            .add_filter("All Files", &["*"])
                            .pick_file()
                        {
                            app.wizard.openvpn_config.config_filename = path.display().to_string();
                        }
                    }
                });
                ui.end_row();

                ui.label("Auth file (optional):");
                ui.horizontal(|ui| {
                    ui.add(
                        egui::TextEdit::singleline(&mut app.wizard.openvpn_config.auth_filename)
                            .hint_text("Optional credentials file")
                            .desired_width(250.0),
                    );
                    if ui.button("📂 Browse...").clicked() {
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter("Text Files", &["txt"])
                            .add_filter("All Files", &["*"])
                            .pick_file()
                        {
                            app.wizard.openvpn_config.auth_filename = path.display().to_string();
                        }
                    }
                });
                ui.end_row();

                ui.label("Route all traffic:");
                ui.checkbox(&mut app.wizard.openvpn_config.route_all_traffic, "");
                ui.end_row();
            });
    }

    fn show_step_confirmation(app: &mut ProxyVmWizardApp, ui: &mut egui::Ui) {
        ui.heading("Step 3: Confirmation");
        ui.add_space(10.0);

        let role = proxy_vm_core::normalize_role_name(&app.wizard.role_name);

        ui.label("The following resources will be created:");
        ui.add_space(10.0);

        egui::Frame::group(ui.style())
            .inner_margin(10.0)
            .show(ui, |ui| {
                ui.label(format!(
                    "📁 Role directory: {}/{}",
                    app.global_config.cfg.root.display(),
                    role
                ));
                ui.label(format!("🌐 Network: {}-inet", role));
                ui.label(format!(
                    "💾 Overlay disk: {}/{}-gw.qcow2",
                    app.global_config.libvirt.images_dir.display(),
                    role
                ));
                ui.label(format!("🖥 Gateway VM: {}-gw", role));

                if let Some(ref id) = app.wizard.selected_gw_template_id {
                    if let Some(template) = app.template_registry.get(id) {
                        ui.label(format!(
                            "📀 Template: {} ({})",
                            template.label, template.os_variant
                        ));
                    }
                }

                ui.add_space(5.0);
                ui.label(format!(
                    "Gateway Mode: {}",
                    app.wizard.gateway_mode.display_name()
                ));

                match app.wizard.gateway_mode {
                    GatewayMode::ProxyChain => {
                        ui.label(format!("Proxy hops: {}", app.wizard.proxy_hops.len()));
                    }
                    GatewayMode::WireGuard => {
                        ui.label(format!(
                            "WireGuard config: /proxy/{}",
                            app.wizard.wireguard_config.config_filename
                        ));
                    }
                    GatewayMode::OpenVpn => {
                        ui.label(format!(
                            "OpenVPN config: /proxy/{}",
                            app.wizard.openvpn_config.config_filename
                        ));
                    }
                }
            });

        ui.add_space(10.0);
        ui.checkbox(
            &mut app.wizard.create_app_vm,
            "Also create an App VM after gateway",
        );
    }

    fn show_step_execution(app: &mut ProxyVmWizardApp, ui: &mut egui::Ui) {
        ui.heading("Step 4: Execution");
        ui.add_space(10.0);

        if app.wizard.is_executing {
            ui.spinner();
            ui.label("Creating resources...");
        }

        ui.add_space(10.0);

        for (i, msg) in app.wizard.execution_messages.iter().enumerate() {
            let is_current = i == app.wizard.execution_step.saturating_sub(1);
            let is_done = i < app.wizard.execution_step;

            let color = if is_done {
                egui::Color32::from_rgb(34, 139, 34)
            } else if is_current && app.wizard.is_executing {
                egui::Color32::from_rgb(100, 149, 237)
            } else {
                egui::Color32::WHITE
            };

            ui.colored_label(color, msg);
        }

        if let Some(ref error) = app.wizard.execution_error.clone() {
            ui.add_space(10.0);
            ui.colored_label(
                egui::Color32::from_rgb(220, 20, 60),
                format!("❌ Error: {}", error),
            );
        }
    }
}
