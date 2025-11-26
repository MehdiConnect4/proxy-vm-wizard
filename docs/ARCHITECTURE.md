# Architecture Overview

This document describes the architecture of Proxy VM Wizard.

## Project Structure

```
proxy-vm-wizard/
├── core/                   # Core library crate
│   └── src/
│       ├── lib.rs          # Library exports
│       ├── model.rs        # Domain models (GatewayMode, ProxyHop, etc.)
│       ├── config.rs       # Configuration management
│       ├── libvirt.rs      # Libvirt/QEMU CLI integration
│       ├── proxy_config.rs # proxy.conf generation
│       ├── vpn_config.rs   # WireGuard/OpenVPN parsing
│       └── error.rs        # Error types
│
├── ui/                     # GUI binary crate
│   └── src/
│       ├── main.rs         # Entry point
│       ├── app.rs          # Application state & logic
│       └── views/          # UI views
│           ├── mod.rs
│           ├── dashboard.rs
│           ├── wizard.rs
│           ├── templates.rs
│           ├── settings.rs
│           └── logs.rs
│
├── assets/                 # Icons, desktop files
├── docs/                   # Documentation
├── scripts/                # Installation scripts
├── flatpak/                # Flatpak packaging
└── .github/workflows/      # CI/CD
```

## Design Principles

### 1. Separation of Concerns

- **core**: Pure library with no GUI dependencies
  - Can be used as a library by other tools
  - All business logic lives here
  - All libvirt/QEMU interaction

- **ui**: Thin GUI layer
  - State management
  - User interaction
  - Visualization

### 2. CLI Tool Wrapping

Instead of using libvirt's C API, we wrap CLI tools:
- `virsh` - VM and network management
- `virt-install` - VM creation
- `qemu-img` - Disk image operations

Benefits:
- Simpler dependency management
- Easier debugging (can run commands manually)
- No unsafe FFI code

### 3. No Shell Invocation

All commands use `std::process::Command` with explicit arguments:

```rust
// Good - direct execution
Command::new("virsh")
    .args(["start", vm_name])
    .output()

// Bad - shell invocation (security risk)
Command::new("sh")
    .args(["-c", format!("virsh start {}", vm_name)])
    .output()
```

### 4. Privilege Escalation

For operations requiring root (writing to `/var/lib/libvirt/images/`):
- Use `pkexec` for graphical password prompt
- Never store passwords
- Minimal privilege scope

## Data Flow

### VM Creation Flow

```
User Input → Wizard UI → WizardState
                            ↓
                    validate_wizard_step()
                            ↓
                    execute_wizard()
                            ↓
    ┌───────────────────────┴───────────────────────┐
    ↓                       ↓                       ↓
ensure_role_network()  create_overlay_disk()  write_config_files()
    ↓                       ↓                       ↓
LibvirtAdapter         LibvirtAdapter         ProxyConfigBuilder
    ↓                       ↓                       ↓
virsh net-*            qemu-img create        proxy.conf
                                              apply-proxy.sh
                            ↓
                    create_gateway_vm()
                            ↓
                    virt-install
```

### Configuration Flow

```
~/.config/proxy-vm-wizard/
├── config.toml         ← GlobalConfig
└── templates.toml      ← TemplateRegistry

~/VMS/VM-Proxy-configs/<role>/
├── role-meta.toml      ← RoleMeta
├── proxy.conf          ← ProxyConfig (generated)
├── apply-proxy.sh      ← Generated script
├── *.conf              ← WireGuard configs
└── *.ovpn              ← OpenVPN configs
```

## Domain Model

### Core Types

```rust
enum GatewayMode {
    ProxyChain,
    WireGuard,
    OpenVpn,
}

struct ProxyHop {
    index: u8,
    proxy_type: ProxyType,  // Socks5 or Http
    host: String,
    port: u16,
    username: Option<String>,
    password: Option<String>,
}

struct ProxyConfig {
    role: String,
    gateway_mode: GatewayMode,
    chain_strategy: ChainStrategy,
    hops: Vec<ProxyHop>,
    wireguard: Option<WireGuardConfig>,
    openvpn: Option<OpenVpnConfig>,
}

struct Template {
    id: String,
    label: String,
    path: PathBuf,
    os_variant: String,
    role_kind: RoleKind,
    default_ram_mb: u32,
}

struct RoleMeta {
    role_name: String,
    gw_template_id: Option<String>,
    app_template_id: Option<String>,
    gateway_mode: GatewayMode,
    app_vm_count: u32,
}
```

## Error Handling

All errors flow through a central `Error` type:

```rust
pub enum Error {
    Io(std::io::Error),
    Config(String),
    Validation(String),
    Command { cmd: String, message: String },
    Libvirt(String),
    Template(String),
    ...
}
```

The UI displays errors with context and recovery options.

## State Management

The `ProxyVmWizardApp` struct holds all application state:

```rust
pub struct ProxyVmWizardApp {
    // Configuration
    global_config: GlobalConfig,
    template_registry: TemplateRegistry,
    libvirt: LibvirtAdapter,
    
    // Navigation
    current_view: View,
    
    // View-specific state
    wizard: WizardState,
    templates_view: TemplatesViewState,
    settings_view: SettingsViewState,
    
    // Runtime state
    discovered_roles: Vec<String>,
    role_vms: HashMap<String, Vec<VmInfo>>,
    
    // Status & logs
    logs: Vec<LogEntry>,
    status_message: Option<(String, StatusLevel)>,
}
```

State is modified through methods that handle validation, side effects, and UI updates.

## Security Considerations

1. **Input Validation**: Role names restricted to `[a-z0-9_-]+`
2. **No Shell**: Direct command execution only
3. **Minimal Privileges**: pkexec for specific operations
4. **No Network**: No external connections except user-initiated tests
5. **Local Storage**: All data in user's home directory


