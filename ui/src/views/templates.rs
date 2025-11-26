//! Templates view - manage qcow2 templates

use crate::app::{ProxyVmWizardApp, StatusLevel};
use eframe::egui;
use proxy_vm_core::{RoleKind, Template};
use std::fs;
use std::path::PathBuf;

pub struct TemplatesView;

impl TemplatesView {
    pub fn show(app: &mut ProxyVmWizardApp, ui: &mut egui::Ui) {
        ui.heading("📁 Template Manager");
        ui.add_space(10.0);

        ui.horizontal(|ui| {
            if ui.button("➕ Add Template").clicked() {
                // First, discover existing qcow2 files in the images directory
                app.templates_view.discovered_qcow2_files =
                    Self::discover_qcow2_files(&app.global_config.libvirt.images_dir);
                // Don't fetch disk-to-VM mapping here - it's slow and blocks UI
                // We'll fetch it lazily only when showing the selection dialog
                app.templates_view.disk_to_vm_map.clear();
                app.templates_view.show_selection_dialog = true;
                app.templates_view.selected_existing_file = None;
                app.templates_view.edit_template_id = None;
            }
        });

        ui.add_space(10.0);

        // Template list
        let templates: Vec<_> = app.template_registry.list().into_iter().cloned().collect();

        if templates.is_empty() {
            ui.vertical_centered(|ui| {
                ui.add_space(50.0);
                ui.label("No templates registered yet.");
                ui.add_space(10.0);
                ui.label("Templates are qcow2 disk images used as base for VMs.");
                ui.label("Click 'Add Template' to register your first template.");
                ui.add_space(20.0);
                ui.label(egui::RichText::new("Recommended:").strong());
                ui.label("• Proxy/Gateway: Debian 12/13 minimal (hardened)");
                ui.label("• App VMs: Debian or Fedora");
            });
        } else {
            egui::ScrollArea::vertical().show(ui, |ui| {
                for template in &templates {
                    Self::show_template_card(app, ui, template);
                    ui.add_space(8.0);
                }
            });
        }

        // Selection dialog (choose existing or browse new)
        if app.templates_view.show_selection_dialog {
            Self::show_selection_dialog(app, ui);
        }

        // Add/Edit dialog
        if app.templates_view.show_add_dialog {
            Self::show_template_dialog(app, ui);
        }

        // Delete confirmation dialog
        if app.templates_view.pending_template_delete.is_some() {
            Self::show_delete_confirmation(app, ui);
        }
    }

    /// Discover all qcow2 files in the images directory
    fn discover_qcow2_files(images_dir: &PathBuf) -> Vec<PathBuf> {
        let mut files = Vec::new();

        // First try direct read (works if user has permissions)
        if let Ok(entries) = fs::read_dir(images_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() {
                    if let Some(ext) = path.extension() {
                        if ext.to_string_lossy().to_lowercase() == "qcow2" {
                            files.push(path);
                        }
                    }
                }
            }
        }

        // If no files found, try using 'ls' command (for permission-restricted directories)
        if files.is_empty() {
            if let Ok(output) = std::process::Command::new("ls")
                .arg("-1")
                .arg(images_dir)
                .output()
            {
                if output.status.success() {
                    let stdout = String::from_utf8_lossy(&output.stdout);
                    for line in stdout.lines() {
                        if line.to_lowercase().ends_with(".qcow2") {
                            files.push(images_dir.join(line));
                        }
                    }
                }
            }
        }

        // If still no files, try with pkexec (will prompt for password)
        if files.is_empty() {
            if let Ok(output) = std::process::Command::new("pkexec")
                .args(["ls", "-1"])
                .arg(images_dir)
                .output()
            {
                if output.status.success() {
                    let stdout = String::from_utf8_lossy(&output.stdout);
                    for line in stdout.lines() {
                        if line.to_lowercase().ends_with(".qcow2") {
                            files.push(images_dir.join(line));
                        }
                    }
                }
            }
        }

        // Sort by filename
        files.sort_by(|a, b| a.file_name().cmp(&b.file_name()));

        files
    }

    fn show_selection_dialog(app: &mut ProxyVmWizardApp, ui: &mut egui::Ui) {
        let is_edit_mode = app.templates_view.edit_template_id.is_some();
        let dialog_title = if is_edit_mode {
            "Edit Template"
        } else {
            "Add Template"
        };

        egui::Window::new(dialog_title)
            .collapsible(false)
            .resizable(true)
            .default_width(500.0)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ui.ctx(), |ui| {
                let prompt = if is_edit_mode {
                    "Choose a disk image for this template:"
                } else {
                    "How would you like to add a template?"
                };
                ui.label(prompt);
                ui.add_space(10.0);

                // Section 1: Existing files in libvirt images directory
                ui.group(|ui| {
                    ui.label(egui::RichText::new("📂 Use Existing Image").strong());
                    ui.label(format!(
                        "From: {}",
                        app.global_config.libvirt.images_dir.display()
                    ));
                    ui.add_space(5.0);

                    if app.templates_view.discovered_qcow2_files.is_empty() {
                        ui.colored_label(
                            egui::Color32::from_rgb(150, 150, 150),
                            "No qcow2 files found in the images directory",
                        );
                    } else {
                        // Get list of already registered paths
                        let registered_paths: Vec<_> = app
                            .template_registry
                            .list()
                            .iter()
                            .map(|t| t.path.clone())
                            .collect();

                        egui::ScrollArea::vertical()
                            .max_height(250.0)
                            .show(ui, |ui| {
                                for path in &app.templates_view.discovered_qcow2_files.clone() {
                                    let filename = path
                                        .file_name()
                                        .map(|n| n.to_string_lossy().to_string())
                                        .unwrap_or_else(|| path.display().to_string());

                                    // Get VMs associated with this image
                                    let vms =
                                        app.libvirt.get_vms_using_image(path).unwrap_or_default();

                                    let is_registered = registered_paths.contains(path);
                                    let is_selected =
                                        app.templates_view.selected_existing_file.as_ref()
                                            == Some(path);

                                    // Create a frame for each image with VM info
                                    egui::Frame::none()
                                        .fill(if is_selected {
                                            egui::Color32::from_rgb(50, 60, 80)
                                        } else {
                                            egui::Color32::TRANSPARENT
                                        })
                                        .rounding(4.0)
                                        .inner_margin(4.0)
                                        .show(ui, |ui| {
                                            ui.vertical(|ui| {
                                                // Show VM names first (if any)
                                                if !vms.is_empty() {
                                                    for vm_name in &vms {
                                                        ui.horizontal(|ui| {
                                                            ui.colored_label(
                                                                egui::Color32::from_rgb(
                                                                    100, 180, 255,
                                                                ),
                                                                format!("🖥 {}", vm_name),
                                                            );
                                                        });
                                                    }
                                                } else {
                                                    ui.colored_label(
                                                        egui::Color32::from_rgb(150, 150, 150),
                                                        "(no registered vm)",
                                                    );
                                                }

                                                // Show filename
                                                ui.horizontal(|ui| {
                                                    let label = if is_registered {
                                                        format!(
                                                            "📋 {} (already registered)",
                                                            filename
                                                        )
                                                    } else {
                                                        format!("💾 {}", filename)
                                                    };

                                                    let response =
                                                        ui.selectable_label(is_selected, label);

                                                    if response.clicked() && !is_registered {
                                                        app.templates_view.selected_existing_file =
                                                            Some(path.clone());
                                                    }

                                                    if is_registered {
                                                        ui.colored_label(
                                                            egui::Color32::from_rgb(100, 180, 100),
                                                            "✓",
                                                        );
                                                    }
                                                });
                                            });
                                        });

                                    ui.add_space(2.0);
                                }
                            });

                        ui.add_space(5.0);

                        // Button to use selected existing file
                        let can_use = app.templates_view.selected_existing_file.is_some();
                        ui.add_enabled_ui(can_use, |ui| {
                            if ui.button("Use Selected Image →").clicked() {
                                if let Some(path) =
                                    &app.templates_view.selected_existing_file.clone()
                                {
                                    // Pre-fill the form with this path
                                    app.templates_view.show_selection_dialog = false;
                                    app.templates_view.show_add_dialog = true;
                                    app.templates_view.edit_template_id = None;
                                    app.templates_view.form_path = path.display().to_string();

                                    // Try to guess a label from filename
                                    let filename = path
                                        .file_stem()
                                        .map(|n| n.to_string_lossy().to_string())
                                        .unwrap_or_default();
                                    app.templates_view.form_label = filename;

                                    // Guess OS variant from filename
                                    let lower = app.templates_view.form_label.to_lowercase();
                                    app.templates_view.form_os_variant = if lower.contains("debian")
                                    {
                                        if lower.contains("13") {
                                            "debian13".to_string()
                                        } else {
                                            "debian12".to_string()
                                        }
                                    } else if lower.contains("fedora") {
                                        if lower.contains("41") {
                                            "fedora41".to_string()
                                        } else {
                                            "fedora40".to_string()
                                        }
                                    } else if lower.contains("ubuntu") {
                                        "ubuntu24.04".to_string()
                                    } else {
                                        "generic".to_string()
                                    };

                                    app.templates_view.form_role_kind = RoleKind::ProxyGateway;
                                    app.templates_view.form_ram_mb = "1024".to_string();
                                    app.templates_view.form_notes = String::new();
                                    app.templates_view.form_error = None;
                                }
                            }
                        });
                    }
                });

                ui.add_space(10.0);

                // Section 2: Browse for a new file
                ui.group(|ui| {
                    ui.label(egui::RichText::new("📁 Import New Image").strong());
                    ui.label("Browse for a qcow2 file from another location.");
                    ui.label(
                        egui::RichText::new("(Will be copied to the images directory)").small(),
                    );
                    ui.add_space(5.0);

                    if ui.button("Browse for File...").clicked() {
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter("QCOW2 Image", &["qcow2"])
                            .pick_file()
                        {
                            app.templates_view.show_selection_dialog = false;
                            app.templates_view.show_add_dialog = true;
                            app.templates_view.edit_template_id = None;
                            app.templates_view.form_path = path.display().to_string();

                            // Try to guess a label from filename
                            let filename = path
                                .file_stem()
                                .map(|n| n.to_string_lossy().to_string())
                                .unwrap_or_default();
                            app.templates_view.form_label = filename;
                            app.templates_view.form_os_variant = "debian12".to_string();
                            app.templates_view.form_role_kind = RoleKind::ProxyGateway;
                            app.templates_view.form_ram_mb = "1024".to_string();
                            app.templates_view.form_notes = String::new();
                            app.templates_view.form_error = None;
                        }
                    }
                });

                ui.add_space(10.0);

                if ui.button("Cancel").clicked() {
                    app.templates_view.show_selection_dialog = false;
                    app.templates_view.selected_existing_file = None;
                }
            });
    }

    fn show_delete_confirmation(app: &mut ProxyVmWizardApp, ui: &mut egui::Ui) {
        let template_id = app.templates_view.pending_template_delete.clone();
        let template_path = app.templates_view.pending_template_delete_path.clone();

        if let (Some(id), Some(path)) = (template_id, template_path) {
            // Get VMs using this image for warning
            let vms_using_image = app.libvirt.get_vms_using_image(&path).unwrap_or_default();

            egui::Window::new("⚠ Confirm Delete")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ui.ctx(), |ui| {
                    ui.label("Are you sure you want to delete this template?");
                    ui.add_space(10.0);

                    ui.group(|ui| {
                        ui.label(egui::RichText::new("This will permanently delete:").strong());
                        ui.add_space(5.0);
                        ui.label("📋 Template from registry");

                        ui.add_space(5.0);
                        ui.checkbox(
                            &mut app.templates_view.delete_image_file,
                            format!("🗑 Also delete image file: {}", path.file_name().unwrap_or_default().to_string_lossy())
                        );

                        if app.templates_view.delete_image_file {
                            ui.label(egui::RichText::new(format!("   Path: {}", path.display())).small());

                            // Show warning if VMs are using this image
                            if !vms_using_image.is_empty() {
                                ui.add_space(5.0);
                                ui.colored_label(
                                    egui::Color32::from_rgb(255, 165, 0),
                                    format!("⚠ Warning: {} VM(s) use this image:", vms_using_image.len())
                                );
                                for vm_name in &vms_using_image {
                                    ui.colored_label(
                                        egui::Color32::from_rgb(255, 165, 0),
                                        format!("   • {}", vm_name)
                                    );
                                }
                                ui.colored_label(
                                    egui::Color32::from_rgb(255, 165, 0),
                                    "   These VMs may stop working!"
                                );
                            }
                        }
                    });

                    ui.add_space(10.0);
                    ui.colored_label(
                        egui::Color32::from_rgb(220, 20, 60),
                        "⚠ This action cannot be undone!"
                    );

                    ui.add_space(10.0);
                    ui.horizontal(|ui| {
                        if ui.button("Cancel").clicked() {
                            app.templates_view.pending_template_delete = None;
                            app.templates_view.pending_template_delete_path = None;
                        }

                        let button_text = if app.templates_view.delete_image_file {
                            "🗑 Delete Template & File"
                        } else {
                            "🗑 Delete Template Only"
                        };

                        if ui.button(egui::RichText::new(button_text).color(egui::Color32::from_rgb(220, 20, 60))).clicked() {
                            // First remove from registry
                            if let Err(e) = app.template_registry.remove(&id) {
                                app.set_status(StatusLevel::Error, format!("Failed to remove from registry: {}", e));
                            } else {
                                // Save registry
                                app.save_template_registry().ok();

                                if app.templates_view.delete_image_file {
                                    // Delete the actual file using pkexec if needed
                                    let delete_result = if path.starts_with("/var/lib") {
                                        app.libvirt.delete_overlay_disk(&path)
                                    } else {
                                        std::fs::remove_file(&path).map_err(proxy_vm_core::Error::Io)
                                    };

                                    match delete_result {
                                        Ok(_) => {
                                            app.set_status(StatusLevel::Success, format!(
                                                "Template and file deleted: {}", path.display()
                                            ));
                                        }
                                        Err(e) => {
                                            app.set_status(StatusLevel::Warning, format!(
                                                "Template removed from registry, but failed to delete file: {}", e
                                            ));
                                        }
                                    }
                                } else {
                                    app.set_status(StatusLevel::Success, format!(
                                        "Template removed from registry (file kept): {}", path.display()
                                    ));
                                }
                            }

                            app.templates_view.pending_template_delete = None;
                            app.templates_view.pending_template_delete_path = None;
                        }
                    });
                });
        }
    }

    fn show_template_card(app: &mut ProxyVmWizardApp, ui: &mut egui::Ui, template: &Template) {
        let exists = template.exists();
        let border_color = if exists {
            egui::Color32::from_rgb(60, 70, 85)
        } else {
            egui::Color32::from_rgb(220, 20, 60)
        };

        egui::Frame::group(ui.style())
            .fill(egui::Color32::from_rgb(30, 35, 45))
            .stroke(egui::Stroke::new(1.0, border_color))
            .rounding(6.0)
            .inner_margin(10.0)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.heading(&template.label);

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        // Delete button - now shows confirmation
                        if ui.small_button("🗑 Remove").clicked() {
                            app.templates_view.pending_template_delete = Some(template.id.clone());
                            app.templates_view.pending_template_delete_path =
                                Some(template.path.clone());
                            app.templates_view.delete_image_file = true; // Default to checked
                        }

                        if ui.small_button("✏ Edit").clicked() {
                            // Discover existing qcow2 files for edit mode too
                            app.templates_view.discovered_qcow2_files =
                                Self::discover_qcow2_files(&app.global_config.libvirt.images_dir);
                            // Don't fetch disk-to-VM mapping here - it's slow and blocks UI
                            app.templates_view.disk_to_vm_map.clear();
                            app.templates_view.show_selection_dialog = true;
                            app.templates_view.selected_existing_file = Some(template.path.clone());
                            app.templates_view.edit_template_id = Some(template.id.clone());
                            // Pre-fill form fields for when user continues to the form
                            app.templates_view.form_label = template.label.clone();
                            app.templates_view.form_path = template.path.display().to_string();
                            app.templates_view.form_os_variant = template.os_variant.clone();
                            app.templates_view.form_role_kind = template.role_kind;
                            app.templates_view.form_ram_mb = template.default_ram_mb.to_string();
                            app.templates_view.form_notes =
                                template.notes.clone().unwrap_or_default();
                            app.templates_view.form_error = None;
                        }
                    });
                });

                ui.add_space(5.0);

                egui::Grid::new(format!("template_info_{}", template.id))
                    .num_columns(2)
                    .spacing([10.0, 4.0])
                    .show(ui, |ui| {
                        ui.label("Path:");
                        ui.horizontal(|ui| {
                            ui.code(template.path.display().to_string());
                            if !exists {
                                ui.colored_label(
                                    egui::Color32::from_rgb(220, 20, 60),
                                    "⚠ Not found",
                                );
                            }
                        });
                        ui.end_row();

                        ui.label("OS Variant:");
                        ui.label(&template.os_variant);
                        ui.end_row();

                        ui.label("Role Kind:");
                        ui.label(template.role_kind.display_name());
                        ui.end_row();

                        ui.label("Default RAM:");
                        ui.label(format!("{} MB", template.default_ram_mb));
                        ui.end_row();

                        if let Some(ref notes) = template.notes {
                            ui.label("Notes:");
                            ui.label(notes);
                            ui.end_row();
                        }
                    });
            });
    }

    fn show_template_dialog(app: &mut ProxyVmWizardApp, ui: &mut egui::Ui) {
        let title = if app.templates_view.edit_template_id.is_some() {
            "Edit Template"
        } else {
            "Add Template"
        };

        egui::Window::new(title)
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ui.ctx(), |ui| {
                egui::Grid::new("template_form")
                    .num_columns(2)
                    .spacing([10.0, 8.0])
                    .show(ui, |ui| {
                        ui.label("Label:");
                        ui.add(
                            egui::TextEdit::singleline(&mut app.templates_view.form_label)
                                .hint_text("e.g., Debian 12 Proxy Base")
                                .desired_width(250.0),
                        );
                        ui.end_row();

                        ui.label("Path:");
                        ui.horizontal(|ui| {
                            ui.add(
                                egui::TextEdit::singleline(&mut app.templates_view.form_path)
                                    .hint_text("/var/lib/libvirt/images/template.qcow2")
                                    .desired_width(300.0),
                            );
                            if ui.button("Browse...").clicked() {
                                if let Some(path) = rfd::FileDialog::new()
                                    .add_filter("QCOW2 Image", &["qcow2"])
                                    .pick_file()
                                {
                                    app.templates_view.form_path = path.display().to_string();
                                }
                            }
                        });
                        ui.end_row();

                        ui.label("OS Variant:");
                        egui::ComboBox::from_id_salt("os_variant_select")
                            .selected_text(&app.templates_view.form_os_variant)
                            .show_ui(ui, |ui| {
                                let variants = [
                                    "debian12",
                                    "debian13",
                                    "debian11",
                                    "fedora40",
                                    "fedora41",
                                    "fedora-rawhide",
                                    "ubuntu22.04",
                                    "ubuntu24.04",
                                    "almalinux9",
                                    "rocky9",
                                    "generic",
                                ];
                                for v in variants {
                                    ui.selectable_value(
                                        &mut app.templates_view.form_os_variant,
                                        v.to_string(),
                                        v,
                                    );
                                }
                            });
                        ui.end_row();

                        ui.label("Role Kind:");
                        ui.horizontal(|ui| {
                            ui.radio_value(
                                &mut app.templates_view.form_role_kind,
                                RoleKind::ProxyGateway,
                                "Proxy/Gateway",
                            );
                            ui.radio_value(
                                &mut app.templates_view.form_role_kind,
                                RoleKind::App,
                                "App",
                            );
                            ui.radio_value(
                                &mut app.templates_view.form_role_kind,
                                RoleKind::Generic,
                                "Generic",
                            );
                        });
                        ui.end_row();

                        ui.label("Default RAM (MB):");
                        ui.add(
                            egui::TextEdit::singleline(&mut app.templates_view.form_ram_mb)
                                .desired_width(80.0),
                        );
                        ui.end_row();

                        ui.label("Notes:");
                        ui.add(
                            egui::TextEdit::multiline(&mut app.templates_view.form_notes)
                                .hint_text("Optional notes...")
                                .desired_width(250.0)
                                .desired_rows(2),
                        );
                        ui.end_row();
                    });

                if let Some(ref error) = app.templates_view.form_error {
                    ui.add_space(5.0);
                    ui.colored_label(egui::Color32::from_rgb(220, 20, 60), error);
                }

                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    if ui.button("Cancel").clicked() {
                        app.templates_view.show_add_dialog = false;
                    }

                    if ui.button("Save").clicked() {
                        Self::save_template(app);
                    }
                });
            });
    }

    fn save_template(app: &mut ProxyVmWizardApp) {
        // Validate
        if app.templates_view.form_label.is_empty() {
            app.templates_view.form_error = Some("Label is required".to_string());
            return;
        }
        if app.templates_view.form_path.is_empty() {
            app.templates_view.form_error = Some("Path is required".to_string());
            return;
        }

        let ram_mb: u32 = match app.templates_view.form_ram_mb.parse() {
            Ok(v) if v >= 128 => v,
            _ => {
                app.templates_view.form_error = Some("RAM must be at least 128 MB".to_string());
                return;
            }
        };

        let mut path = PathBuf::from(&app.templates_view.form_path);

        // Check if file exists
        if !path.exists() {
            app.templates_view.form_error = Some(format!("File not found: {}", path.display()));
            return;
        }

        // If template is not in the images directory, copy it there automatically
        let images_dir = &app.global_config.libvirt.images_dir;
        if !app.libvirt.is_in_images_dir(&path, images_dir) {
            // Check if a file with the same name already exists in images_dir
            let filename = path.file_name().unwrap_or_default();
            let dest_path = images_dir.join(filename);

            if dest_path.exists() {
                // File already exists, just use it
                app.log(
                    StatusLevel::Success,
                    format!("Using existing template at {}", dest_path.display()),
                );
                path = dest_path;
            } else {
                // Need to copy
                match app.libvirt.copy_template_to_images_dir(&path, images_dir) {
                    Ok(new_path) => {
                        app.log(
                            StatusLevel::Success,
                            format!("Copied template to {}", new_path.display()),
                        );
                        path = new_path;
                    }
                    Err(e) => {
                        app.templates_view.form_error = Some(format!(
                            "Failed to copy template: {}. A password dialog should appear - please authenticate.",
                            e
                        ));
                        return;
                    }
                }
            }
        }

        let template = Template {
            id: app
                .templates_view
                .edit_template_id
                .clone()
                .unwrap_or_else(|| app.template_registry.generate_id()),
            label: app.templates_view.form_label.clone(),
            path,
            os_variant: app.templates_view.form_os_variant.clone(),
            role_kind: app.templates_view.form_role_kind,
            default_ram_mb: ram_mb,
            notes: if app.templates_view.form_notes.is_empty() {
                None
            } else {
                Some(app.templates_view.form_notes.clone())
            },
        };

        let result = if app.templates_view.edit_template_id.is_some() {
            app.template_registry.update(template)
        } else {
            app.template_registry.add(template)
        };

        match result {
            Ok(_) => {
                if let Err(e) = app.save_template_registry() {
                    app.set_status(
                        StatusLevel::Error,
                        format!("Failed to save registry: {}", e),
                    );
                } else {
                    app.set_status(StatusLevel::Success, "Template saved successfully");
                    app.templates_view.show_add_dialog = false;
                }
            }
            Err(e) => {
                app.templates_view.form_error = Some(e.to_string());
            }
        }
    }
}
