//! Main application state and logic

use eframe::egui;
use proxy_vm_core::{
    GlobalConfig, TemplateRegistry, RoleMeta, LibvirtAdapter, ProxyConfig,
    ProxyConfigBuilder, RoleKind, GatewayMode, ProxyHop, ProxyType,
    WireGuardConfig, OpenVpnConfig, VmInfo,
    validate_role_name, normalize_role_name, config::discover_roles,
    AuthState, EncryptionManager,
};
use std::path::PathBuf;
use std::collections::HashMap;
use std::sync::mpsc::{channel, Receiver, Sender};

use crate::views::{View, DashboardView, WizardView, TemplatesView, SettingsView, LogsView};

/// Authentication screen state
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum AuthScreen {
    #[default]
    None,
    /// First launch - setup password
    Setup,
    /// Subsequent launches - login
    Login,
}

/// Authentication view state
#[derive(Default)]
pub struct AuthViewState {
    pub screen: AuthScreen,
    pub password: String,
    pub password_confirm: String,
    pub error: Option<String>,
    pub show_password: bool,
}

/// Message types for async operations (reserved for future background tasks)
#[derive(Debug)]
#[allow(dead_code)]
pub enum AsyncMessage {
    VmListRefreshed(Vec<VmInfo>),
    RolesDiscovered(Vec<String>),
    OperationSuccess(String),
    OperationError(String),
    ConnectionTestResult { index: usize, success: bool, message: String },
}

/// Main application state
pub struct ProxyVmWizardApp {
    // Authentication
    pub auth_view: AuthViewState,
    pub encryption: Option<EncryptionManager>,

    // Configuration
    pub global_config: GlobalConfig,
    pub template_registry: TemplateRegistry,
    pub libvirt: LibvirtAdapter,

    // Navigation
    pub current_view: View,
    pub previous_view: Option<View>,

    // Dashboard state
    pub discovered_roles: Vec<String>,
    pub role_vms: HashMap<String, Vec<VmInfo>>,
    pub last_refresh: Option<std::time::Instant>,

    // Wizard state
    pub wizard: WizardState,

    // Templates view state
    pub templates_view: TemplatesViewState,

    // Settings view state  
    pub settings_view: SettingsViewState,

    // Logs
    pub logs: Vec<LogEntry>,
    pub max_logs: usize,

    // Async communication (reserved for future background tasks)
    #[allow(dead_code)]
    pub async_tx: Sender<AsyncMessage>,
    #[allow(dead_code)]
    pub async_rx: Receiver<AsyncMessage>,

    // Status
    pub status_message: Option<(String, StatusLevel)>,
    pub prereq_error: Option<String>,

    // Pending confirmations
    pub pending_role_delete: Option<String>,

    // Config editor state (for editing role configs from dashboard)
    pub editing_role_config: Option<String>,
    pub config_editor: ConfigEditorState,
}

/// State for editing a role's gateway configuration
#[derive(Default, Clone)]
pub struct ConfigEditorState {
    pub gateway_mode: GatewayMode,
    pub proxy_hops: Vec<ProxyHopEntry>,
    pub wireguard_config: WireGuardConfigEntry,
    pub openvpn_config: OpenVpnConfigEntry,
    pub error: Option<String>,
    pub restart_after_save: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatusLevel {
    #[allow(dead_code)]
    Info,
    Success,
    Warning,
    Error,
}

#[derive(Debug, Clone)]
pub struct LogEntry {
    pub timestamp: chrono::DateTime<chrono::Local>,
    pub level: StatusLevel,
    pub message: String,
}

/// Wizard state for creating/editing roles
#[derive(Default)]
pub struct WizardState {
    pub step: WizardStep,
    pub mode: WizardMode,

    // Step 1: Role basics
    pub role_name: String,
    pub role_name_error: Option<String>,
    pub selected_gw_template_id: Option<String>,
    pub selected_app_template_id: Option<String>,
    pub selected_disp_template_id: Option<String>,

    // Step 2: Gateway mode
    pub gateway_mode: GatewayMode,
    #[allow(dead_code)]
    pub previous_gateway_mode: Option<GatewayMode>,
    pub pending_mode_change: Option<GatewayMode>,
    pub proxy_hops: Vec<ProxyHopEntry>,
    pub wireguard_config: WireGuardConfigEntry,
    pub openvpn_config: OpenVpnConfigEntry,

    // Step 3: Confirmation
    pub create_app_vm: bool,

    // Execution state
    pub is_executing: bool,
    pub execution_step: usize,
    pub execution_messages: Vec<String>,
    pub execution_error: Option<String>,

    // Cleanup tracking - what was created during this wizard run
    pub created_network: Option<String>,
    pub created_overlay: Option<std::path::PathBuf>,
    pub created_vm: Option<String>,
    pub created_role_dir: Option<std::path::PathBuf>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WizardStep {
    #[default]
    RoleBasics,
    GatewayConfig,
    Confirmation,
    Execution,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WizardMode {
    #[default]
    Create,
    #[allow(dead_code)]
    Edit,
}

#[derive(Default, Clone)]
pub struct ProxyHopEntry {
    pub proxy_type: ProxyType,
    pub host: String,
    pub port: String,
    pub username: String,
    pub password: String,
    pub label: String,
    pub test_status: Option<bool>,
    pub test_message: Option<String>,
}

#[derive(Default, Clone)]
pub struct WireGuardConfigEntry {
    pub config_filename: String,
    pub interface_name: String,
    pub route_all_traffic: bool,
}

#[derive(Default, Clone)]
pub struct OpenVpnConfigEntry {
    pub config_filename: String,
    pub auth_filename: String,
    pub route_all_traffic: bool,
}

/// Templates view state
#[derive(Default)]
pub struct TemplatesViewState {
    pub show_add_dialog: bool,
    pub edit_template_id: Option<String>,
    
    // Selection mode - first ask to pick existing or browse new
    pub show_selection_dialog: bool,
    pub discovered_qcow2_files: Vec<std::path::PathBuf>,
    pub selected_existing_file: Option<std::path::PathBuf>,
    
    // Map of disk paths to VM names that use them
    pub disk_to_vm_map: HashMap<std::path::PathBuf, Vec<String>>,
    
    // Delete confirmation
    pub pending_template_delete: Option<String>,  // template ID to delete
    pub pending_template_delete_path: Option<std::path::PathBuf>,  // path to delete
    pub delete_image_file: bool,  // Whether to also delete the image file (default: true)
    
    // Form fields
    pub form_label: String,
    pub form_path: String,
    pub form_os_variant: String,
    pub form_role_kind: RoleKind,
    pub form_ram_mb: String,
    pub form_notes: String,
    pub form_error: Option<String>,
}

/// Settings view state
#[derive(Default)]
pub struct SettingsViewState {
    pub cfg_root: String,
    pub images_dir: String,
    pub lan_net: String,
    pub gateway_ram: String,
    pub app_ram: String,
    pub disp_ram: String,
    pub debian_variant: String,
    pub fedora_variant: String,
    pub error: Option<String>,
    pub saved: bool,
}

impl ProxyVmWizardApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // Set up fonts
        let mut fonts = egui::FontDefinitions::default();
        fonts.font_data.insert(
            "JetBrainsMono".to_owned(),
            egui::FontData::from_static(include_bytes!(
                "../assets/JetBrainsMono-Regular.ttf"
            )),
        );
        fonts
            .families
            .entry(egui::FontFamily::Monospace)
            .or_default()
            .insert(0, "JetBrainsMono".to_owned());
        cc.egui_ctx.set_fonts(fonts);

        // Configure style
        let mut style = (*cc.egui_ctx.style()).clone();
        style.spacing.item_spacing = egui::vec2(8.0, 6.0);
        style.spacing.button_padding = egui::vec2(12.0, 6.0);
        cc.egui_ctx.set_style(style);

        // Create async channel
        let (async_tx, async_rx) = channel();

        // Initialize libvirt adapter
        let libvirt = LibvirtAdapter::new();

        // Check if auth is set up
        let auth_screen = if AuthState::is_setup() {
            AuthScreen::Login
        } else {
            AuthScreen::Setup
        };

        // Create app with minimal state - actual config loading happens after authentication
        Self {
            auth_view: AuthViewState {
                screen: auth_screen,
                ..Default::default()
            },
            encryption: None,
            global_config: GlobalConfig::default(),
            template_registry: TemplateRegistry::default(),
            libvirt,
            current_view: View::Dashboard,
            previous_view: None,
            discovered_roles: Vec::new(),
            role_vms: HashMap::new(),
            last_refresh: None,
            wizard: WizardState::default(),
            templates_view: TemplatesViewState::default(),
            settings_view: SettingsViewState::default(),
            logs: Vec::new(),
            max_logs: 500,
            async_tx,
            async_rx,
            status_message: None,
            prereq_error: None,
            pending_role_delete: None,
            editing_role_config: None,
            config_editor: ConfigEditorState::default(),
        }
    }

    /// Initialize the app after successful authentication
    fn initialize_after_auth(&mut self) {
        // Check prerequisites
        self.prereq_error = match self.libvirt.check_prerequisites() {
            Ok(_) => match self.libvirt.check_libvirt_access() {
                Ok(_) => None,
                Err(e) => Some(e.to_string()),
            },
            Err(e) => Some(e.to_string()),
        };

        // Collect any warnings to log after loading
        let mut warnings: Vec<String> = Vec::new();

        // Load config (encrypted or create new)
        if let Some(ref encryption) = self.encryption.clone() {
            // Try to load encrypted config
            match GlobalConfig::load_encrypted(encryption) {
                Ok(config) => self.global_config = config,
                Err(_) => {
                    // Try plain config (might exist from before encryption)
                    self.global_config = GlobalConfig::load_or_default().unwrap_or_default();
                    // Save as encrypted
                    if let Err(e) = self.global_config.save_encrypted(encryption) {
                        warnings.push(format!("Failed to encrypt config: {}", e));
                    }
                }
            }

            // Try to load encrypted template registry
            match TemplateRegistry::load_encrypted(encryption) {
                Ok(registry) => self.template_registry = registry,
                Err(_) => {
                    // Try plain registry
                    self.template_registry = TemplateRegistry::load_or_default().unwrap_or_default();
                    // Save as encrypted
                    if let Err(e) = self.template_registry.save_encrypted(encryption) {
                        warnings.push(format!("Failed to encrypt templates: {}", e));
                    }
                }
            }
        } else {
            // No encryption - load normally
            self.global_config = GlobalConfig::load_or_default().unwrap_or_default();
            self.template_registry = TemplateRegistry::load_or_default().unwrap_or_default();
        }

        // Log any warnings
        for warning in warnings {
            self.log(StatusLevel::Warning, warning);
        }

        // Discover roles
        self.discovered_roles = discover_roles(&self.global_config.cfg.root).unwrap_or_default();

        // Initialize settings view state from config
        self.settings_view = SettingsViewState {
            cfg_root: self.global_config.cfg.root.display().to_string(),
            images_dir: self.global_config.libvirt.images_dir.display().to_string(),
            lan_net: self.global_config.libvirt.lan_net.clone(),
            gateway_ram: self.global_config.defaults.gateway_ram_mb.to_string(),
            app_ram: self.global_config.defaults.app_ram_mb.to_string(),
            disp_ram: self.global_config.defaults.disp_ram_mb.to_string(),
            debian_variant: self.global_config.defaults.debian_os_variant.clone(),
            fedora_variant: self.global_config.defaults.fedora_os_variant.clone(),
            error: None,
            saved: false,
        };

        // Initial refresh
        self.refresh_vms();
    }

    /// Handle password setup
    fn setup_password(&mut self) -> bool {
        if self.auth_view.password.len() < 8 {
            self.auth_view.error = Some("Password must be at least 8 characters".to_string());
            return false;
        }

        if self.auth_view.password != self.auth_view.password_confirm {
            self.auth_view.error = Some("Passwords do not match".to_string());
            return false;
        }

        // Create auth state
        match AuthState::create(&self.auth_view.password) {
            Ok(auth_state) => {
                // Save auth state
                if let Err(e) = auth_state.save() {
                    self.auth_view.error = Some(format!("Failed to save auth: {}", e));
                    return false;
                }

                // Create encryption manager
                match EncryptionManager::from_password(&self.auth_view.password, &auth_state) {
                    Ok(encryption) => {
                        self.encryption = Some(encryption);
                        self.auth_view.screen = AuthScreen::None;
                        self.auth_view.password.clear();
                        self.auth_view.password_confirm.clear();
                        self.initialize_after_auth();
                        true
                    }
                    Err(e) => {
                        self.auth_view.error = Some(format!("Failed to create encryption: {}", e));
                        false
                    }
                }
            }
            Err(e) => {
                self.auth_view.error = Some(format!("Failed to create auth: {}", e));
                false
            }
        }
    }

    /// Handle login
    fn login(&mut self) -> bool {
        match AuthState::load() {
            Ok(auth_state) => {
                // Verify password
                match auth_state.verify_password(&self.auth_view.password) {
                    Ok(true) => {
                        // Create encryption manager
                        match EncryptionManager::from_password(&self.auth_view.password, &auth_state) {
                            Ok(encryption) => {
                                self.encryption = Some(encryption);
                                self.auth_view.screen = AuthScreen::None;
                                self.auth_view.password.clear();
                                self.initialize_after_auth();
                                true
                            }
                            Err(e) => {
                                self.auth_view.error = Some(format!("Encryption error: {}", e));
                                false
                            }
                        }
                    }
                    Ok(false) => {
                        self.auth_view.error = Some("Incorrect password".to_string());
                        false
                    }
                    Err(e) => {
                        self.auth_view.error = Some(format!("Verification error: {}", e));
                        false
                    }
                }
            }
            Err(e) => {
                self.auth_view.error = Some(format!("Failed to load auth: {}", e));
                false
            }
        }
    }

    /// Show the password setup screen
    fn show_setup_screen(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(80.0);
                
                ui.heading("🔐 Proxy VM Wizard");
                ui.add_space(10.0);
                ui.label("Welcome! Please set up a password to secure your configuration.");
                ui.add_space(5.0);
                ui.label(egui::RichText::new("This password will encrypt all your settings and templates.")
                    .small()
                    .color(egui::Color32::from_rgb(150, 150, 150)));
                
                ui.add_space(30.0);
                
                egui::Frame::group(ui.style())
                    .fill(egui::Color32::from_rgb(30, 35, 45))
                    .rounding(8.0)
                    .inner_margin(20.0)
                    .show(ui, |ui| {
                        ui.set_width(350.0);
                        
                        ui.label("Create Password:");
                        ui.add_space(5.0);
                        let password_edit = egui::TextEdit::singleline(&mut self.auth_view.password)
                            .password(!self.auth_view.show_password)
                            .hint_text("Enter password (min 8 characters)")
                            .desired_width(300.0);
                        ui.add(password_edit);
                        
                        ui.add_space(10.0);
                        
                        ui.label("Confirm Password:");
                        ui.add_space(5.0);
                        let confirm_edit = egui::TextEdit::singleline(&mut self.auth_view.password_confirm)
                            .password(!self.auth_view.show_password)
                            .hint_text("Confirm password")
                            .desired_width(300.0);
                        ui.add(confirm_edit);
                        
                        ui.add_space(10.0);
                        ui.checkbox(&mut self.auth_view.show_password, "Show password");
                        
                        if let Some(ref error) = self.auth_view.error {
                            ui.add_space(10.0);
                            ui.colored_label(egui::Color32::from_rgb(220, 20, 60), error);
                        }
                        
                        ui.add_space(20.0);
                        
                        ui.horizontal(|ui| {
                            if ui.button("🔐 Create Password & Continue").clicked() {
                                self.setup_password();
                            }
                        });
                    });
                
                ui.add_space(20.0);
                ui.label(egui::RichText::new("⚠ Remember this password! It cannot be recovered if lost.")
                    .color(egui::Color32::from_rgb(255, 165, 0)));
            });
        });
    }

    /// Show the login screen
    fn show_login_screen(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(100.0);
                
                ui.heading("🔐 Proxy VM Wizard");
                ui.add_space(10.0);
                ui.label("Enter your password to unlock the application.");
                
                ui.add_space(30.0);
                
                egui::Frame::group(ui.style())
                    .fill(egui::Color32::from_rgb(30, 35, 45))
                    .rounding(8.0)
                    .inner_margin(20.0)
                    .show(ui, |ui| {
                        ui.set_width(350.0);
                        
                        ui.label("Password:");
                        ui.add_space(5.0);
                        
                        let password_edit = egui::TextEdit::singleline(&mut self.auth_view.password)
                            .password(!self.auth_view.show_password)
                            .hint_text("Enter your password")
                            .desired_width(300.0);
                        let response = ui.add(password_edit);
                        
                        // Submit on Enter
                        if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                            self.login();
                        }
                        
                        ui.add_space(10.0);
                        ui.checkbox(&mut self.auth_view.show_password, "Show password");
                        
                        if let Some(ref error) = self.auth_view.error {
                            ui.add_space(10.0);
                            ui.colored_label(egui::Color32::from_rgb(220, 20, 60), error);
                        }
                        
                        ui.add_space(20.0);
                        
                        if ui.button("🔓 Unlock").clicked() {
                            self.login();
                        }
                    });
            });
        });
    }

    pub fn log(&mut self, level: StatusLevel, message: impl Into<String>) {
        let entry = LogEntry {
            timestamp: chrono::Local::now(),
            level,
            message: message.into(),
        };
        self.logs.push(entry);
        if self.logs.len() > self.max_logs {
            self.logs.remove(0);
        }
    }

    pub fn set_status(&mut self, level: StatusLevel, message: impl Into<String>) {
        let msg = message.into();
        self.log(level, &msg);
        self.status_message = Some((msg, level));
    }

    pub fn clear_status(&mut self) {
        self.status_message = None;
    }

    pub fn navigate_to(&mut self, view: View) {
        self.previous_view = Some(self.current_view);
        self.current_view = view;
    }

    pub fn refresh_vms(&mut self) {
        self.role_vms.clear();
        
        // Refresh roles
        self.discovered_roles = discover_roles(&self.global_config.cfg.root).unwrap_or_default();

        // Get all VMs
        match self.libvirt.list_vms(None) {
            Ok(vms) => {
                for vm in vms {
                    if let Some(role) = &vm.role {
                        self.role_vms
                            .entry(role.clone())
                            .or_default()
                            .push(vm);
                    }
                }
            }
            Err(e) => {
                self.log(StatusLevel::Error, format!("Failed to list VMs: {}", e));
            }
        }

        self.last_refresh = Some(std::time::Instant::now());
    }

    pub fn start_vm(&mut self, name: &str) {
        // First check current state
        if let Ok(Some(info)) = self.libvirt.get_vm_info(name) {
            if info.state.is_running() {
                self.set_status(StatusLevel::Warning, format!("VM '{}' is already running", name));
                self.refresh_vms();
                return;
            }
        }
        
        match self.libvirt.start_vm(name) {
            Ok(_) => {
                self.set_status(StatusLevel::Success, format!("Started VM: {}", name));
                self.refresh_vms();
            }
            Err(e) => {
                let msg = e.to_string();
                if msg.contains("already running") || msg.contains("is running") {
                    self.set_status(StatusLevel::Warning, format!("VM '{}' is already running", name));
                } else {
                    self.set_status(StatusLevel::Error, format!("Failed to start VM: {}", e));
                }
                self.refresh_vms();
            }
        }
    }

    pub fn stop_vm(&mut self, name: &str) {
        // First check current state
        if let Ok(Some(info)) = self.libvirt.get_vm_info(name) {
            if !info.state.is_running() {
                self.set_status(StatusLevel::Warning, format!("VM '{}' is not running", name));
                self.refresh_vms();
                return;
            }
        }
        
        match self.libvirt.stop_vm(name) {
            Ok(_) => {
                self.set_status(StatusLevel::Success, format!("Stopping VM: {}", name));
                self.refresh_vms();
            }
            Err(e) => {
                let msg = e.to_string();
                if msg.contains("not running") || msg.contains("domain is not running") {
                    self.set_status(StatusLevel::Warning, format!("VM '{}' is already stopped", name));
                } else {
                    self.set_status(StatusLevel::Error, format!("Failed to stop VM: {}", e));
                }
                self.refresh_vms();
            }
        }
    }

    pub fn reset_wizard(&mut self) {
        // Clean up any partial resources from previous wizard run
        self.cleanup_wizard_resources();
        
        self.wizard = WizardState::default();
        // Add initial proxy hop
        self.wizard.proxy_hops.push(ProxyHopEntry::default());
    }

    /// Check if proxy chain has any data entered
    pub fn proxy_chain_has_data(&self) -> bool {
        self.wizard.proxy_hops.iter().any(|hop| {
            !hop.host.is_empty() || !hop.port.is_empty()
        })
    }

    /// Check if wireguard config has data
    pub fn wireguard_has_data(&self) -> bool {
        !self.wizard.wireguard_config.config_filename.is_empty()
    }

    /// Check if openvpn config has data  
    pub fn openvpn_has_data(&self) -> bool {
        !self.wizard.openvpn_config.config_filename.is_empty()
    }

    /// Check if current mode has data
    pub fn current_mode_has_data(&self, mode: GatewayMode) -> bool {
        match mode {
            GatewayMode::ProxyChain => self.proxy_chain_has_data(),
            GatewayMode::WireGuard => self.wireguard_has_data(),
            GatewayMode::OpenVpn => self.openvpn_has_data(),
        }
    }

    /// Clear data for a specific mode
    pub fn clear_mode_data(&mut self, mode: GatewayMode) {
        match mode {
            GatewayMode::ProxyChain => {
                self.wizard.proxy_hops.clear();
                self.wizard.proxy_hops.push(ProxyHopEntry::default());
            }
            GatewayMode::WireGuard => {
                self.wizard.wireguard_config = WireGuardConfigEntry::default();
            }
            GatewayMode::OpenVpn => {
                self.wizard.openvpn_config = OpenVpnConfigEntry::default();
            }
        }
    }

    /// Request to change gateway mode - will prompt if current mode has data
    pub fn request_mode_change(&mut self, new_mode: GatewayMode) {
        if new_mode == self.wizard.gateway_mode {
            return;
        }

        // Check if current mode has data that would be lost
        if self.current_mode_has_data(self.wizard.gateway_mode) {
            // Need confirmation
            self.wizard.pending_mode_change = Some(new_mode);
        } else {
            // No data to lose, just switch
            self.wizard.gateway_mode = new_mode;
        }
    }

    /// Confirm mode change - clears old mode data and switches
    pub fn confirm_mode_change(&mut self) {
        if let Some(new_mode) = self.wizard.pending_mode_change.take() {
            self.clear_mode_data(self.wizard.gateway_mode);
            self.wizard.gateway_mode = new_mode;
        }
    }

    /// Cancel mode change
    pub fn cancel_mode_change(&mut self) {
        self.wizard.pending_mode_change = None;
    }

    /// Start editing a role's gateway configuration
    pub fn start_editing_role_config(&mut self, role: &str) {
        // Load current config from role metadata
        self.config_editor = ConfigEditorState::default();
        self.config_editor.restart_after_save = true;

        // Try to load from role meta
        if let Ok(meta) = RoleMeta::load(&self.global_config.cfg.root, role) {
            self.config_editor.gateway_mode = meta.gateway_mode;
        }

        // Try to parse existing proxy.conf to load current settings
        let role_dir = self.global_config.role_dir(role);
        let conf_path = role_dir.join("proxy.conf");
        if let Ok(content) = std::fs::read_to_string(&conf_path) {
            self.parse_proxy_conf_into_editor(&content);
        }

        // Ensure at least one proxy hop exists
        if self.config_editor.proxy_hops.is_empty() {
            self.config_editor.proxy_hops.push(ProxyHopEntry::default());
        }

        self.editing_role_config = Some(role.to_string());
    }

    /// Parse proxy.conf content into the config editor state
    fn parse_proxy_conf_into_editor(&mut self, content: &str) {
        let mut values: std::collections::HashMap<String, String> = std::collections::HashMap::new();
        
        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            if let Some((key, value)) = line.split_once('=') {
                values.insert(key.to_string(), value.to_string());
            }
        }

        // Parse gateway mode
        if let Some(mode) = values.get("GATEWAY_MODE") {
            self.config_editor.gateway_mode = match mode.as_str() {
                "PROXY_CHAIN" => GatewayMode::ProxyChain,
                "WIREGUARD" => GatewayMode::WireGuard,
                "OPENVPN" => GatewayMode::OpenVpn,
                _ => GatewayMode::ProxyChain,
            };
        }

        // Parse proxy chain
        if let Some(count_str) = values.get("PROXY_COUNT") {
            if let Ok(count) = count_str.parse::<usize>() {
                self.config_editor.proxy_hops.clear();
                for i in 1..=count {
                    let mut hop = ProxyHopEntry::default();
                    if let Some(t) = values.get(&format!("PROXY_{}_TYPE", i)) {
                        hop.proxy_type = if t == "HTTP" { ProxyType::Http } else { ProxyType::Socks5 };
                    }
                    if let Some(h) = values.get(&format!("PROXY_{}_HOST", i)) {
                        hop.host = h.clone();
                    }
                    if let Some(p) = values.get(&format!("PROXY_{}_PORT", i)) {
                        hop.port = p.clone();
                    }
                    if let Some(u) = values.get(&format!("PROXY_{}_USER", i)) {
                        hop.username = u.clone();
                    }
                    if let Some(p) = values.get(&format!("PROXY_{}_PASS", i)) {
                        hop.password = p.clone();
                    }
                    if let Some(l) = values.get(&format!("PROXY_{}_LABEL", i)) {
                        hop.label = l.clone();
                    }
                    self.config_editor.proxy_hops.push(hop);
                }
            }
        }

        // Parse WireGuard config
        if let Some(path) = values.get("WG_CONFIG_PATH") {
            self.config_editor.wireguard_config.config_filename = path.replace("/proxy/", "");
        }
        if let Some(iface) = values.get("WG_INTERFACE_NAME") {
            self.config_editor.wireguard_config.interface_name = iface.clone();
        }
        if let Some(route) = values.get("WG_ROUTE_ALL_TRAFFIC") {
            self.config_editor.wireguard_config.route_all_traffic = route == "true";
        }

        // Parse OpenVPN config
        if let Some(path) = values.get("OPENVPN_CONFIG_PATH") {
            self.config_editor.openvpn_config.config_filename = path.replace("/proxy/", "");
        }
        if let Some(auth) = values.get("OPENVPN_AUTH_FILE") {
            self.config_editor.openvpn_config.auth_filename = auth.replace("/proxy/", "");
        }
        if let Some(route) = values.get("OPENVPN_ROUTE_ALL_TRAFFIC") {
            self.config_editor.openvpn_config.route_all_traffic = route == "true";
        }
    }

    /// Save the edited configuration and optionally restart the gateway VM
    pub fn save_role_config(&mut self) {
        let role = match &self.editing_role_config {
            Some(r) => r.clone(),
            None => return,
        };

        let role_dir = self.global_config.role_dir(&role);
        let gw_name = format!("{}-gw", role);

        // Build proxy config from editor state
        let mut config = ProxyConfig::new(role.clone(), self.config_editor.gateway_mode);
        
        match self.config_editor.gateway_mode {
            GatewayMode::ProxyChain => {
                for (i, hop) in self.config_editor.proxy_hops.iter().enumerate() {
                    if hop.host.is_empty() {
                        continue;
                    }
                    let port = hop.port.parse().unwrap_or(1080);
                    let mut proxy_hop = ProxyHop::new(
                        (i + 1) as u8,
                        hop.proxy_type,
                        hop.host.clone(),
                        port,
                    );
                    if !hop.username.is_empty() {
                        proxy_hop.username = Some(hop.username.clone());
                    }
                    if !hop.password.is_empty() {
                        proxy_hop.password = Some(hop.password.clone());
                    }
                    if !hop.label.is_empty() {
                        proxy_hop.label = Some(hop.label.clone());
                    }
                    config.add_hop(proxy_hop);
                }
            }
            GatewayMode::WireGuard => {
                config.wireguard = Some(WireGuardConfig {
                    config_path: format!("/proxy/{}", self.config_editor.wireguard_config.config_filename),
                    interface_name: if self.config_editor.wireguard_config.interface_name.is_empty() {
                        "wg0".to_string()
                    } else {
                        self.config_editor.wireguard_config.interface_name.clone()
                    },
                    route_all_traffic: self.config_editor.wireguard_config.route_all_traffic,
                });
            }
            GatewayMode::OpenVpn => {
                config.openvpn = Some(OpenVpnConfig {
                    config_path: format!("/proxy/{}", self.config_editor.openvpn_config.config_filename),
                    auth_file: if self.config_editor.openvpn_config.auth_filename.is_empty() {
                        None
                    } else {
                        Some(format!("/proxy/{}", self.config_editor.openvpn_config.auth_filename))
                    },
                    route_all_traffic: self.config_editor.openvpn_config.route_all_traffic,
                });
            }
        }

        // Write config files
        if let Err(e) = ProxyConfigBuilder::write_config_files(&config, &role_dir) {
            self.config_editor.error = Some(format!("Failed to save config: {}", e));
            return;
        }

        // Update role meta
        if let Ok(mut meta) = RoleMeta::load(&self.global_config.cfg.root, &role) {
            meta.gateway_mode = self.config_editor.gateway_mode;
            meta.save(&self.global_config.cfg.root).ok();
        }

        // Restart VM if requested
        if self.config_editor.restart_after_save {
            // Stop the VM
            self.libvirt.stop_vm(&gw_name).ok();
            // Wait a moment then start
            std::thread::sleep(std::time::Duration::from_millis(500));
            if let Err(e) = self.libvirt.start_vm(&gw_name) {
                self.log(StatusLevel::Warning, format!("Config saved but VM restart failed: {}", e));
            } else {
                self.log(StatusLevel::Success, format!("Config saved and VM '{}' restarting", gw_name));
            }
        } else {
            self.set_status(StatusLevel::Success, "Configuration saved. Restart VM to apply changes.");
        }

        self.editing_role_config = None;
        self.refresh_vms();
    }

    /// Cancel editing role config
    pub fn cancel_editing_role_config(&mut self) {
        self.editing_role_config = None;
    }

    /// Clean up any resources created during a failed/cancelled wizard run
    pub fn cleanup_wizard_resources(&mut self) {
        // Clean up VM first
        if let Some(ref vm_name) = self.wizard.created_vm.take() {
            self.log(StatusLevel::Warning, format!("Cleaning up VM '{}'...", vm_name));
            self.libvirt.destroy_vm(vm_name).ok();
            self.libvirt.undefine_vm(vm_name).ok();
        }

        // Clean up overlay disk
        if let Some(ref overlay_path) = self.wizard.created_overlay.take() {
            if overlay_path.exists() {
                self.log(StatusLevel::Warning, format!("Cleaning up overlay disk '{}'...", overlay_path.display()));
                self.libvirt.delete_overlay_disk(overlay_path).ok();
            }
        }

        // Clean up network (only if we created it)
        if let Some(ref net_name) = self.wizard.created_network.take() {
            self.log(StatusLevel::Warning, format!("Cleaning up network '{}'...", net_name));
            self.libvirt.destroy_network(net_name).ok();
        }

        // Clean up role directory (only if it's empty or only has our config files)
        if let Some(ref role_dir) = self.wizard.created_role_dir.take() {
            if role_dir.exists() {
                // Only delete if directory is relatively empty (our files only)
                if let Ok(entries) = std::fs::read_dir(role_dir) {
                    let count = entries.count();
                    if count <= 3 {  // proxy.conf, apply-proxy.sh, role-meta.toml
                        self.log(StatusLevel::Warning, format!("Cleaning up role directory '{}'...", role_dir.display()));
                        std::fs::remove_dir_all(role_dir).ok();
                    }
                }
            }
        }
    }

    pub fn start_create_role_wizard(&mut self) {
        self.reset_wizard();
        self.wizard.mode = WizardMode::Create;
        self.navigate_to(View::Wizard);
    }

    #[allow(dead_code)]
    pub fn start_edit_role_wizard(&mut self, role: &str) {
        self.reset_wizard();
        self.wizard.mode = WizardMode::Edit;
        self.wizard.role_name = role.to_string();
        
        // Load existing role meta if available
        if let Ok(meta) = RoleMeta::load(&self.global_config.cfg.root, role) {
            self.wizard.selected_gw_template_id = meta.gw_template_id;
            self.wizard.selected_app_template_id = meta.app_template_id;
            self.wizard.selected_disp_template_id = meta.disp_template_id;
            self.wizard.gateway_mode = meta.gateway_mode;
        }

        self.navigate_to(View::Wizard);
    }

    pub fn validate_wizard_step(&mut self) -> bool {
        match self.wizard.step {
            WizardStep::RoleBasics => {
                let name = normalize_role_name(&self.wizard.role_name);
                if let Err(e) = validate_role_name(&name) {
                    self.wizard.role_name_error = Some(e);
                    return false;
                }

                // Check if role already exists (for create mode)
                if self.wizard.mode == WizardMode::Create {
                    let role_dir = self.global_config.role_dir(&name);
                    if role_dir.exists() {
                        self.wizard.role_name_error = Some("Role already exists".to_string());
                        return false;
                    }
                }

                // Must have gateway template selected
                if self.wizard.selected_gw_template_id.is_none() {
                    self.wizard.role_name_error = Some("Please select a gateway template".to_string());
                    return false;
                }

                self.wizard.role_name_error = None;
                true
            }
            WizardStep::GatewayConfig => {
                match self.wizard.gateway_mode {
                    GatewayMode::ProxyChain => {
                        if self.wizard.proxy_hops.is_empty() {
                            return false;
                        }
                        // Validate all hops
                        for hop in &self.wizard.proxy_hops {
                            if hop.host.is_empty() {
                                return false;
                            }
                            if hop.port.parse::<u16>().is_err() {
                                return false;
                            }
                        }
                        true
                    }
                    GatewayMode::WireGuard => {
                        !self.wizard.wireguard_config.config_filename.is_empty()
                    }
                    GatewayMode::OpenVpn => {
                        !self.wizard.openvpn_config.config_filename.is_empty()
                    }
                }
            }
            _ => true,
        }
    }

    pub fn wizard_next_step(&mut self) {
        if !self.validate_wizard_step() {
            return;
        }

        self.wizard.step = match self.wizard.step {
            WizardStep::RoleBasics => WizardStep::GatewayConfig,
            WizardStep::GatewayConfig => WizardStep::Confirmation,
            WizardStep::Confirmation => {
                self.execute_wizard();
                WizardStep::Execution
            }
            WizardStep::Execution => WizardStep::Execution,
        };
    }

    pub fn wizard_prev_step(&mut self) {
        self.wizard.step = match self.wizard.step {
            WizardStep::RoleBasics => WizardStep::RoleBasics,
            WizardStep::GatewayConfig => WizardStep::RoleBasics,
            WizardStep::Confirmation => WizardStep::GatewayConfig,
            WizardStep::Execution => WizardStep::Confirmation,
        };
    }

    pub fn execute_wizard(&mut self) {
        self.wizard.is_executing = true;
        self.wizard.execution_step = 0;
        self.wizard.execution_messages.clear();
        self.wizard.execution_error = None;
        
        // Reset cleanup tracking
        self.wizard.created_network = None;
        self.wizard.created_overlay = None;
        self.wizard.created_vm = None;
        self.wizard.created_role_dir = None;

        let role = normalize_role_name(&self.wizard.role_name);
        let role_dir = self.global_config.role_dir(&role);
        let role_net = format!("{}-inet", role);
        let gw_name = format!("{}-gw", role);

        // Step 1: Validate global config
        self.wizard.execution_messages.push("Validating configuration...".to_string());
        if let Err(e) = self.global_config.validate() {
            self.wizard.execution_error = Some(format!("Config validation failed: {}", e));
            self.wizard.is_executing = false;
            return;
        }
        self.wizard.execution_step = 1;

        // Step 2: Validate template
        self.wizard.execution_messages.push("Checking template...".to_string());
        let template_id = match self.wizard.selected_gw_template_id.as_ref() {
            Some(id) => id.clone(),
            None => {
                self.wizard.execution_error = Some("No gateway template selected".to_string());
                self.wizard.is_executing = false;
                return;
            }
        };
        let template = match self.template_registry.get(&template_id) {
            Some(t) => t.clone(),
            None => {
                self.wizard.execution_error = Some("Gateway template not found".to_string());
                self.wizard.is_executing = false;
                return;
            }
        };
        if let Err(e) = template.validate() {
            self.wizard.execution_error = Some(format!("Template error: {}", e));
            self.wizard.is_executing = false;
            return;
        }
        self.wizard.execution_step = 2;

        // Step 3: Ensure LAN network exists
        self.wizard.execution_messages.push(format!("Checking LAN network '{}'...", self.global_config.libvirt.lan_net));
        if let Err(e) = self.libvirt.ensure_lan_net_exists(&self.global_config.libvirt.lan_net) {
            self.wizard.execution_error = Some(e.to_string());
            self.wizard.is_executing = false;
            return;
        }
        self.wizard.execution_step = 3;

        // Step 4: Create role network
        self.wizard.execution_messages.push(format!("Creating role network '{}'...", role_net));
        match self.libvirt.ensure_role_network(&role) {
            Ok(created) => {
                if created {
                    self.wizard.execution_messages.push(format!("Created network '{}'", role_net));
                    // Track for cleanup
                    self.wizard.created_network = Some(role_net.clone());
                } else {
                    self.wizard.execution_messages.push(format!("Network '{}' already exists", role_net));
                }
            }
            Err(e) => {
                self.wizard.execution_error = Some(format!("Failed to create network: {}", e));
                self.wizard.is_executing = false;
                self.cleanup_wizard_resources();
                return;
            }
        };
        self.wizard.execution_step = 4;

        // Step 5: Copy VPN config files if needed and generate proxy config
        self.wizard.execution_messages.push("Generating proxy configuration...".to_string());
        
        // Create role directory first
        let role_dir_existed = role_dir.exists();
        if let Err(e) = std::fs::create_dir_all(&role_dir) {
            self.wizard.execution_error = Some(format!("Failed to create role directory: {}", e));
            self.wizard.is_executing = false;
            self.cleanup_wizard_resources();
            return;
        }
        // Track for cleanup only if we created it
        if !role_dir_existed {
            self.wizard.created_role_dir = Some(role_dir.clone());
        }

        // Copy WireGuard config if it's a file path
        if self.wizard.gateway_mode == GatewayMode::WireGuard {
            let wg_path = std::path::Path::new(&self.wizard.wireguard_config.config_filename);
            if wg_path.exists() && wg_path.is_file() {
                if let Some(filename) = wg_path.file_name() {
                    let dest = role_dir.join(filename);
                    if let Err(e) = std::fs::copy(wg_path, &dest) {
                        self.wizard.execution_error = Some(format!("Failed to copy WireGuard config: {}", e));
                        self.wizard.is_executing = false;
                        self.cleanup_wizard_resources();
                        return;
                    }
                    self.wizard.execution_messages.push(format!("Copied WireGuard config to {}", dest.display()));
                    // Update to just the filename for the config
                    self.wizard.wireguard_config.config_filename = filename.to_string_lossy().to_string();
                }
            }
        }

        // Copy OpenVPN configs if they're file paths
        if self.wizard.gateway_mode == GatewayMode::OpenVpn {
            let ovpn_path = std::path::Path::new(&self.wizard.openvpn_config.config_filename);
            if ovpn_path.exists() && ovpn_path.is_file() {
                if let Some(filename) = ovpn_path.file_name() {
                    let dest = role_dir.join(filename);
                    if let Err(e) = std::fs::copy(ovpn_path, &dest) {
                        self.wizard.execution_error = Some(format!("Failed to copy OpenVPN config: {}", e));
                        self.wizard.is_executing = false;
                        self.cleanup_wizard_resources();
                        return;
                    }
                    self.wizard.execution_messages.push(format!("Copied OpenVPN config to {}", dest.display()));
                    self.wizard.openvpn_config.config_filename = filename.to_string_lossy().to_string();
                }
            }

            // Copy auth file if provided
            if !self.wizard.openvpn_config.auth_filename.is_empty() {
                let auth_path = std::path::Path::new(&self.wizard.openvpn_config.auth_filename);
                if auth_path.exists() && auth_path.is_file() {
                    if let Some(filename) = auth_path.file_name() {
                        let dest = role_dir.join(filename);
                        if let Err(e) = std::fs::copy(auth_path, &dest) {
                            self.log(StatusLevel::Warning, format!("Failed to copy auth file: {}", e));
                        } else {
                            self.wizard.execution_messages.push(format!("Copied auth file to {}", dest.display()));
                            self.wizard.openvpn_config.auth_filename = filename.to_string_lossy().to_string();
                        }
                    }
                }
            }
        }

        let proxy_config = self.build_proxy_config();
        if let Err(e) = ProxyConfigBuilder::write_config_files(&proxy_config, &role_dir) {
            self.wizard.execution_error = Some(format!("Failed to write config: {}", e));
            self.wizard.is_executing = false;
            self.cleanup_wizard_resources();
            return;
        }
        self.wizard.execution_step = 5;

        // Step 6: Create overlay disk
        self.wizard.execution_messages.push("Creating overlay disk...".to_string());
        let overlay_path = self.libvirt.gateway_overlay_path(&self.global_config.libvirt.images_dir, &role);
        if let Err(e) = self.libvirt.create_overlay_disk(&template.path, &overlay_path) {
            self.wizard.execution_error = Some(format!("Failed to create overlay: {}", e));
            self.wizard.is_executing = false;
            self.cleanup_wizard_resources();
            return;
        }
        // Track overlay for cleanup
        self.wizard.created_overlay = Some(overlay_path.clone());
        self.wizard.execution_step = 6;

        // Step 7: Create gateway VM
        self.wizard.execution_messages.push(format!("Creating gateway VM '{}'...", gw_name));
        let ram_mb = template.default_ram_mb.max(self.global_config.defaults.gateway_ram_mb);
        if let Err(e) = self.libvirt.create_gateway_vm(
            &gw_name,
            &overlay_path,
            &self.global_config.libvirt.lan_net,
            &role_net,
            &role_dir,
            &template.os_variant,
            ram_mb,
        ) {
            self.wizard.execution_error = Some(format!("Failed to create VM: {}", e));
            self.wizard.is_executing = false;
            self.cleanup_wizard_resources();
            return;
        }
        // Track VM for cleanup (though at this point we're almost done)
        self.wizard.created_vm = Some(gw_name.clone());
        self.wizard.execution_step = 7;

        // Step 8: Save role metadata
        self.wizard.execution_messages.push("Saving role metadata...".to_string());
        let mut meta = RoleMeta::new(role.clone());
        meta.gw_template_id = self.wizard.selected_gw_template_id.clone();
        meta.app_template_id = self.wizard.selected_app_template_id.clone();
        meta.disp_template_id = self.wizard.selected_disp_template_id.clone();
        meta.gateway_mode = self.wizard.gateway_mode;
        if let Err(e) = meta.save(&self.global_config.cfg.root) {
            self.log(StatusLevel::Warning, format!("Failed to save role metadata: {}", e));
        }
        self.wizard.execution_step = 8;

        // Step 9: Create App VM if requested
        if self.wizard.create_app_vm {
            if let Some(ref app_template_id) = self.wizard.selected_app_template_id {
                if let Some(app_template) = self.template_registry.get(app_template_id).cloned() {
                    self.wizard.execution_messages.push("Creating App VM...".to_string());
                    
                    // Load and update meta for app VM numbering
                    let mut meta = RoleMeta::load(&self.global_config.cfg.root, &role)
                        .unwrap_or_else(|_| RoleMeta::new(role.clone()));
                    let app_num = meta.next_app_number();
                    let app_vm_name = meta.app_vm_name(app_num);
                    
                    // Create app overlay
                    let app_overlay = self.libvirt.app_overlay_path(&self.global_config.libvirt.images_dir, &role, app_num);
                    if let Err(e) = self.libvirt.create_overlay_disk(&app_template.path, &app_overlay) {
                        self.log(StatusLevel::Warning, format!("Failed to create App VM overlay: {}", e));
                    } else {
                        // Create app VM
                        let app_ram = app_template.default_ram_mb.max(self.global_config.defaults.app_ram_mb);
                        if let Err(e) = self.libvirt.create_app_vm(
                            &app_vm_name,
                            &app_overlay,
                            &role_net,
                            &app_template.os_variant,
                            app_ram,
                            None,
                        ) {
                            self.log(StatusLevel::Warning, format!("Failed to create App VM: {}", e));
                            self.libvirt.delete_overlay_disk(&app_overlay).ok();
                        } else {
                            self.wizard.execution_messages.push(format!("✓ Created App VM '{}'", app_vm_name));
                            // Save updated meta
                            meta.save(&self.global_config.cfg.root).ok();
                        }
                    }
                } else {
                    self.log(StatusLevel::Warning, "App template not found, skipping App VM creation");
                }
            } else {
                self.log(StatusLevel::Warning, "No App template selected, skipping App VM creation");
            }
        }

        self.wizard.execution_messages.push("✓ Role created successfully!".to_string());
        self.wizard.is_executing = false;
        
        // Clear cleanup tracking - everything succeeded!
        self.wizard.created_network = None;
        self.wizard.created_overlay = None;
        self.wizard.created_vm = None;
        self.wizard.created_role_dir = None;
        
        self.log(StatusLevel::Success, format!("Created role '{}' with gateway VM '{}'", role, gw_name));

        // Refresh VM list
        self.refresh_vms();
    }

    fn build_proxy_config(&self) -> ProxyConfig {
        let role = normalize_role_name(&self.wizard.role_name);
        let mut config = ProxyConfig::new(role, self.wizard.gateway_mode);

        match self.wizard.gateway_mode {
            GatewayMode::ProxyChain => {
                for (i, hop_entry) in self.wizard.proxy_hops.iter().enumerate() {
                    let port = hop_entry.port.parse().unwrap_or(1080);
                    let mut hop = ProxyHop::new(
                        (i + 1) as u8,
                        hop_entry.proxy_type,
                        hop_entry.host.clone(),
                        port,
                    );
                    if !hop_entry.username.is_empty() {
                        hop.username = Some(hop_entry.username.clone());
                    }
                    if !hop_entry.password.is_empty() {
                        hop.password = Some(hop_entry.password.clone());
                    }
                    if !hop_entry.label.is_empty() {
                        hop.label = Some(hop_entry.label.clone());
                    }
                    config.add_hop(hop);
                }
            }
            GatewayMode::WireGuard => {
                config.wireguard = Some(WireGuardConfig {
                    config_path: format!("/proxy/{}", self.wizard.wireguard_config.config_filename),
                    interface_name: if self.wizard.wireguard_config.interface_name.is_empty() {
                        "wg0".to_string()
                    } else {
                        self.wizard.wireguard_config.interface_name.clone()
                    },
                    route_all_traffic: self.wizard.wireguard_config.route_all_traffic,
                });
            }
            GatewayMode::OpenVpn => {
                config.openvpn = Some(OpenVpnConfig {
                    config_path: format!("/proxy/{}", self.wizard.openvpn_config.config_filename),
                    auth_file: if self.wizard.openvpn_config.auth_filename.is_empty() {
                        None
                    } else {
                        Some(format!("/proxy/{}", self.wizard.openvpn_config.auth_filename))
                    },
                    route_all_traffic: self.wizard.openvpn_config.route_all_traffic,
                });
            }
        }

        config
    }

    pub fn test_proxy_connection(&mut self, index: usize) {
        if index >= self.wizard.proxy_hops.len() {
            return;
        }

        let hop = &self.wizard.proxy_hops[index];
        let host = hop.host.clone();
        let port: u16 = hop.port.parse().unwrap_or(0);

        if host.is_empty() || port == 0 {
            self.wizard.proxy_hops[index].test_status = Some(false);
            self.wizard.proxy_hops[index].test_message = Some("Invalid host or port".to_string());
            return;
        }

        match self.libvirt.test_tcp_connection(&host, port) {
            Ok(_) => {
                self.wizard.proxy_hops[index].test_status = Some(true);
                self.wizard.proxy_hops[index].test_message = Some("Connection successful".to_string());
            }
            Err(e) => {
                self.wizard.proxy_hops[index].test_status = Some(false);
                self.wizard.proxy_hops[index].test_message = Some(e.to_string());
            }
        }
    }

    pub fn create_app_vm(&mut self, role: &str) {
        let role_net = format!("{}-inet", role);

        // Get app template
        let template_id = match RoleMeta::load(&self.global_config.cfg.root, role) {
            Ok(meta) => meta.app_template_id,
            Err(_) => None,
        };

        let template = match template_id.and_then(|id| self.template_registry.get(&id)) {
            Some(t) => t,
            None => {
                self.set_status(StatusLevel::Error, "No app template configured for this role");
                return;
            }
        };

        // Get next app number
        let mut meta = RoleMeta::load(&self.global_config.cfg.root, role)
            .unwrap_or_else(|_| RoleMeta::new(role.to_string()));
        let app_num = meta.next_app_number();
        let vm_name = meta.app_vm_name(app_num);

        // Create overlay
        let overlay_path = self.libvirt.app_overlay_path(&self.global_config.libvirt.images_dir, role, app_num);
        if let Err(e) = self.libvirt.create_overlay_disk(&template.path, &overlay_path) {
            self.set_status(StatusLevel::Error, format!("Failed to create overlay: {}", e));
            return;
        }

        // Create VM
        let ram_mb = template.default_ram_mb.max(self.global_config.defaults.app_ram_mb);
        if let Err(e) = self.libvirt.create_app_vm(
            &vm_name,
            &overlay_path,
            &role_net,
            &template.os_variant,
            ram_mb,
            None,
        ) {
            self.libvirt.delete_overlay_disk(&overlay_path).ok();
            self.set_status(StatusLevel::Error, format!("Failed to create VM: {}", e));
            return;
        }

        // Save updated meta
        if let Err(e) = meta.save(&self.global_config.cfg.root) {
            self.log(StatusLevel::Warning, format!("Failed to save role metadata: {}", e));
        }

        self.set_status(StatusLevel::Success, format!("Created app VM: {}", vm_name));
        self.refresh_vms();
    }

    pub fn delete_role(&mut self, role: &str) {
        self.log(StatusLevel::Warning, format!("Deleting role '{}' and all associated resources...", role));
        
        let role_net = format!("{}-inet", role);
        let gw_name = format!("{}-gw", role);
        let role_dir = self.global_config.role_dir(role);
        
        // Get all VMs for this role
        let vms = self.role_vms.get(role).cloned().unwrap_or_default();
        
        // Delete all VMs (gateway, app VMs, disposables)
        for vm in &vms {
            self.log(StatusLevel::Warning, format!("Removing VM '{}'...", vm.name));
            // Destroy if running
            self.libvirt.destroy_vm(&vm.name).ok();
            // Undefine
            self.libvirt.undefine_vm(&vm.name).ok();
        }
        
        // Also try to delete the gateway VM by name pattern in case it wasn't in the list
        self.libvirt.destroy_vm(&gw_name).ok();
        self.libvirt.undefine_vm(&gw_name).ok();
        
        // Delete overlay disks
        let gw_overlay = self.libvirt.gateway_overlay_path(&self.global_config.libvirt.images_dir, role);
        if gw_overlay.exists() {
            self.log(StatusLevel::Warning, format!("Removing overlay disk '{}'...", gw_overlay.display()));
            self.libvirt.delete_overlay_disk(&gw_overlay).ok();
        }
        
        // Delete app VM overlays (try a few numbers)
        for i in 1..=20 {
            let app_overlay = self.libvirt.app_overlay_path(&self.global_config.libvirt.images_dir, role, i);
            if app_overlay.exists() {
                self.libvirt.delete_overlay_disk(&app_overlay).ok();
            }
        }
        
        // Delete disposable overlay directory
        let disp_dir = role_dir.join("disposable");
        if disp_dir.exists() {
            std::fs::remove_dir_all(&disp_dir).ok();
        }
        
        // Destroy and undefine the role network
        self.log(StatusLevel::Warning, format!("Removing network '{}'...", role_net));
        self.libvirt.destroy_network(&role_net).ok();
        
        // Delete role config directory
        if role_dir.exists() {
            self.log(StatusLevel::Warning, format!("Removing config directory '{}'...", role_dir.display()));
            std::fs::remove_dir_all(&role_dir).ok();
        }
        
        self.set_status(StatusLevel::Success, format!("Deleted role '{}' and all associated resources", role));
        self.refresh_vms();
    }

    pub fn launch_disposable_vm(&mut self, role: &str) {
        let role_net = format!("{}-inet", role);

        // Get disposable template (fallback to app template)
        let template_id = match RoleMeta::load(&self.global_config.cfg.root, role) {
            Ok(meta) => meta.disp_template_id.or(meta.app_template_id),
            Err(_) => None,
        };

        let template = match template_id.and_then(|id| self.template_registry.get(&id)) {
            Some(t) => t,
            None => {
                self.set_status(StatusLevel::Error, "No disposable/app template configured for this role");
                return;
            }
        };

        // Generate name and overlay path
        let timestamp = chrono::Local::now().format("%Y%m%d-%H%M%S");
        let vm_name = format!("disp-{}-{}", role, timestamp);
        let overlay_path = self.libvirt.disposable_overlay_path(&self.global_config.cfg.root, role);

        // Create overlay
        if let Err(e) = self.libvirt.create_overlay_disk(&template.path, &overlay_path) {
            self.set_status(StatusLevel::Error, format!("Failed to create overlay: {}", e));
            return;
        }

        // Create transient VM
        let ram_mb = template.default_ram_mb.max(self.global_config.defaults.disp_ram_mb);
        if let Err(e) = self.libvirt.create_disposable_vm(
            &vm_name,
            &overlay_path,
            &role_net,
            &template.os_variant,
            ram_mb,
        ) {
            self.libvirt.delete_overlay_disk(&overlay_path).ok();
            self.set_status(StatusLevel::Error, format!("Failed to create disposable VM: {}", e));
            return;
        }

        self.set_status(StatusLevel::Success, format!("Launched disposable VM: {}", vm_name));
        self.refresh_vms();
    }

    pub fn save_settings(&mut self) {
        // Parse and validate
        let gateway_ram = match self.settings_view.gateway_ram.parse::<u32>() {
            Ok(v) if v >= 128 => v,
            _ => {
                self.settings_view.error = Some("Gateway RAM must be at least 128 MB".to_string());
                return;
            }
        };
        let app_ram = match self.settings_view.app_ram.parse::<u32>() {
            Ok(v) if v >= 256 => v,
            _ => {
                self.settings_view.error = Some("App RAM must be at least 256 MB".to_string());
                return;
            }
        };
        let disp_ram = match self.settings_view.disp_ram.parse::<u32>() {
            Ok(v) if v >= 256 => v,
            _ => {
                self.settings_view.error = Some("Disposable RAM must be at least 256 MB".to_string());
                return;
            }
        };

        // Update config
        self.global_config.cfg.root = PathBuf::from(&self.settings_view.cfg_root);
        self.global_config.libvirt.images_dir = PathBuf::from(&self.settings_view.images_dir);
        self.global_config.libvirt.lan_net = self.settings_view.lan_net.clone();
        self.global_config.defaults.gateway_ram_mb = gateway_ram;
        self.global_config.defaults.app_ram_mb = app_ram;
        self.global_config.defaults.disp_ram_mb = disp_ram;
        self.global_config.defaults.debian_os_variant = self.settings_view.debian_variant.clone();
        self.global_config.defaults.fedora_os_variant = self.settings_view.fedora_variant.clone();

        // Save (encrypted if encryption is available)
        let save_result = if let Some(ref encryption) = self.encryption {
            self.global_config.save_encrypted(encryption)
        } else {
            self.global_config.save()
        };

        match save_result {
            Ok(_) => {
                self.settings_view.error = None;
                self.settings_view.saved = true;
                self.set_status(StatusLevel::Success, "Settings saved");
            }
            Err(e) => {
                self.settings_view.error = Some(format!("Failed to save: {}", e));
            }
        }
    }

    /// Save template registry (encrypted if encryption is available)
    pub fn save_template_registry(&mut self) -> proxy_vm_core::Result<()> {
        if let Some(ref encryption) = self.encryption {
            self.template_registry.save_encrypted(encryption)
        } else {
            self.template_registry.save()
        }
    }
}

impl eframe::App for ProxyVmWizardApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Show authentication screen if needed
        match self.auth_view.screen {
            AuthScreen::Setup => {
                self.show_setup_screen(ctx);
                return;
            }
            AuthScreen::Login => {
                self.show_login_screen(ctx);
                return;
            }
            AuthScreen::None => {}
        }

        // Handle async messages
        while let Ok(msg) = self.async_rx.try_recv() {
            match msg {
                AsyncMessage::OperationSuccess(m) => {
                    self.set_status(StatusLevel::Success, m);
                }
                AsyncMessage::OperationError(e) => {
                    self.set_status(StatusLevel::Error, e);
                }
                AsyncMessage::ConnectionTestResult { index, success, message } => {
                    if index < self.wizard.proxy_hops.len() {
                        self.wizard.proxy_hops[index].test_status = Some(success);
                        self.wizard.proxy_hops[index].test_message = Some(message);
                    }
                }
                _ => {}
            }
        }

        // Prerequisite error modal
        if let Some(ref error) = self.prereq_error {
            egui::Window::new("⚠ Prerequisite Error")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui| {
                    ui.label(error);
                    ui.add_space(10.0);
                    ui.label("Please ensure libvirt is installed and you have access.");
                    ui.label("Try: sudo usermod -aG libvirt $USER");
                });
            return;
        }

        // Top panel with navigation
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading("🖥 Proxy VM Wizard");
                ui.separator();

                ui.selectable_value(&mut self.current_view, View::Dashboard, "📊 Dashboard");
                ui.selectable_value(&mut self.current_view, View::Wizard, "🧙 Wizard");
                ui.selectable_value(&mut self.current_view, View::Templates, "📁 Templates");
                ui.selectable_value(&mut self.current_view, View::Settings, "⚙ Settings");
                ui.selectable_value(&mut self.current_view, View::Logs, "📝 Logs");

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("🔄 Refresh").clicked() {
                        self.refresh_vms();
                    }
                });
            });
        });

        // Status bar
        if let Some((ref msg, level)) = self.status_message.clone() {
            egui::TopBottomPanel::bottom("status_panel").show(ctx, |ui| {
                ui.horizontal(|ui| {
                    let color = match level {
                        StatusLevel::Info => egui::Color32::from_rgb(100, 149, 237),
                        StatusLevel::Success => egui::Color32::from_rgb(34, 139, 34),
                        StatusLevel::Warning => egui::Color32::from_rgb(255, 165, 0),
                        StatusLevel::Error => egui::Color32::from_rgb(220, 20, 60),
                    };
                    ui.colored_label(color, msg);
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.small_button("✕").clicked() {
                            self.clear_status();
                        }
                    });
                });
            });
        }

        // Main content
        egui::CentralPanel::default().show(ctx, |ui| {
            match self.current_view {
                View::Dashboard => DashboardView::show(self, ui),
                View::Wizard => WizardView::show(self, ui),
                View::Templates => TemplatesView::show(self, ui),
                View::Settings => SettingsView::show(self, ui),
                View::Logs => LogsView::show(self, ui),
            }
        });

        // Request repaint for real-time updates
        ctx.request_repaint_after(std::time::Duration::from_secs(5));
    }
}

