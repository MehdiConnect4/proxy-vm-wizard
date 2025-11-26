//! Proxy VM Wizard - GUI Application
//!
//! A production-grade Rust GUI wizard for managing proxy/gateway VMs,
//! app VMs, and disposable VMs using libvirt/QEMU.

mod app;
mod views;

use app::ProxyVmWizardApp;
use eframe::egui;

fn main() -> eframe::Result<()> {
    // Set up logging
    env_logger::init();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1200.0, 800.0])
            .with_min_inner_size([900.0, 600.0])
            .with_title("Proxy VM Wizard"),
        ..Default::default()
    };

    eframe::run_native(
        "Proxy VM Wizard",
        options,
        Box::new(|cc| Ok(Box::new(ProxyVmWizardApp::new(cc)))),
    )
}
