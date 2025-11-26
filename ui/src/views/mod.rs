//! View modules for the Proxy VM Wizard GUI

mod dashboard;
mod wizard;
mod templates;
mod settings;
mod logs;

pub use dashboard::DashboardView;
pub use wizard::WizardView;
pub use templates::TemplatesView;
pub use settings::SettingsView;
pub use logs::LogsView;

/// Navigation views
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum View {
    #[default]
    Dashboard,
    Wizard,
    Templates,
    Settings,
    Logs,
}




