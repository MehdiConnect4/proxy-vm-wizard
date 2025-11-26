//! View modules for the Proxy VM Wizard GUI

mod dashboard;
mod logs;
mod settings;
mod templates;
mod wizard;

pub use dashboard::DashboardView;
pub use logs::LogsView;
pub use settings::SettingsView;
pub use templates::TemplatesView;
pub use wizard::WizardView;

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
