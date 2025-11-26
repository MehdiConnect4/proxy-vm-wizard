//! Logs view - scrolling log display

use eframe::egui;
use crate::app::{ProxyVmWizardApp, StatusLevel};

pub struct LogsView;

impl LogsView {
    pub fn show(app: &mut ProxyVmWizardApp, ui: &mut egui::Ui) {
        ui.heading("📝 Logs");
        ui.add_space(10.0);

        ui.horizontal(|ui| {
            if ui.button("🗑 Clear Logs").clicked() {
                app.logs.clear();
            }
            ui.label(format!("{} entries", app.logs.len()));
        });

        ui.add_space(10.0);

        if app.logs.is_empty() {
            ui.vertical_centered(|ui| {
                ui.add_space(50.0);
                ui.label("No log entries yet.");
            });
            return;
        }

        egui::ScrollArea::vertical()
            .auto_shrink([false; 2])
            .stick_to_bottom(true)
            .show(ui, |ui| {
                for entry in &app.logs {
                    let (icon, color) = match entry.level {
                        StatusLevel::Info => ("ℹ", egui::Color32::from_rgb(100, 149, 237)),
                        StatusLevel::Success => ("✓", egui::Color32::from_rgb(34, 139, 34)),
                        StatusLevel::Warning => ("⚠", egui::Color32::from_rgb(255, 165, 0)),
                        StatusLevel::Error => ("✗", egui::Color32::from_rgb(220, 20, 60)),
                    };

                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new(entry.timestamp.format("%H:%M:%S").to_string())
                                .color(egui::Color32::GRAY)
                                .monospace()
                        );
                        ui.colored_label(color, icon);
                        ui.label(&entry.message);
                    });
                }
            });
    }
}





