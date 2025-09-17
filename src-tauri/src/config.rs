use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::PathBuf;
use crate::error::{MessengerError, Result};

/// Main application configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub app: AppSettings,
    pub network: NetworkConfig,
    pub security: SecurityConfig,
    pub ui: UiConfig,
    pub storage: StorageConfig,
    pub logging: LoggingConfig,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            app: AppSettings::default(),
            network: NetworkConfig::default(),
            security: SecurityConfig::default(),
            ui: UiConfig::default(),
            storage: StorageConfig::default(),
            logging: LoggingConfig::default(),
        }
    }
}

/// Application settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    pub name: String,
    pub version: String,
    pub debug: bool,
    pub auto_start: bool,
    pub minimize_to_tray: bool,
    pub close_to_tray: bool,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            name: "TCP Messenger".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            debug: false,
            auto_start: false,
            minimize_to_tray: true,
            close_to_tray: true,
        }
    }
}

/// Network configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfig {
    pub server: ServerConfig,
    pub client: ClientConfig,
    pub discovery: DiscoveryConfig,
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            server: ServerConfig::default(),
            client: ClientConfig::default(),
            discovery: DiscoveryConfig::default(),
        }
    }
}

/// Server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub port_range: (u16, u16),
    pub max_clients: u32,
    pub heartbeat_interval: u64, // seconds
    pub connection_timeout: u64, // seconds
    pub message_timeout: u64, // seconds
    pub auto_start: bool,
    pub bind_all_interfaces: bool,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            port_range: (8000, 8100),
            max_clients: 1,
            heartbeat_interval: 30,
            connection_timeout: 60,
            message_timeout: 5,
            auto_start: false,
            bind_all_interfaces: true,
        }
    }
}

/// Client configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientConfig {
    pub connection_timeout: u64, // seconds
    pub retry_attempts: u32,
    pub retry_delay: u64, // milliseconds
    pub auto_reconnect: bool,
    pub reconnect_delay: u64, // seconds
    pub keep_alive: bool,
}

impl Default for ClientConfig {
    fn default() -> Self {
        Self {
            connection_timeout: 10,
            retry_attempts: 3,
            retry_delay: 1000,
            auto_reconnect: true,
            reconnect_delay: 5,
            keep_alive: true,
        }
    }
}

/// Network discovery configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveryConfig {
    pub enabled: bool,
    pub broadcast_interval: u64, // seconds
    pub listen_port: u16,
    pub service_name: String,
    pub timeout: u64, // seconds
}

impl Default for DiscoveryConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            broadcast_interval: 30,
            listen_port: 9000,
            service_name: "tcp-messenger".to_string(),
            timeout: 5,
        }
    }
}

/// Security configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    pub encryption_enabled: bool,
    pub key_rotation_interval: u32, // number of messages
    pub max_message_size: usize, // bytes
    pub allowed_file_types: HashSet<String>,
    pub max_file_size: u64, // bytes
    pub require_authentication: bool,
    pub session_timeout: u64, // seconds
}

impl Default for SecurityConfig {
    fn default() -> Self {
        let mut allowed_types = HashSet::new();
        allowed_types.insert(".txt".to_string());
        allowed_types.insert(".pdf".to_string());
        allowed_types.insert(".jpg".to_string());
        allowed_types.insert(".jpeg".to_string());
        allowed_types.insert(".png".to_string());
        allowed_types.insert(".gif".to_string());
        allowed_types.insert(".zip".to_string());
        allowed_types.insert(".doc".to_string());
        allowed_types.insert(".docx".to_string());

        Self {
            encryption_enabled: true,
            key_rotation_interval: 100,
            max_message_size: 1024 * 1024, // 1MB
            allowed_file_types: allowed_types,
            max_file_size: 100 * 1024 * 1024, // 100MB
            require_authentication: true,
            session_timeout: 3600, // 1 hour
        }
    }
}

/// UI configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiConfig {
    pub theme: Theme,
    pub font_size: FontSize,
    pub show_timestamps: bool,
    pub show_message_status: bool,
    pub sound_notifications: bool,
    pub notification_sound: String,
    pub auto_scroll: bool,
    pub message_grouping: bool,
    pub compact_mode: bool,
    pub window_size: (u32, u32),
    pub window_position: Option<(i32, i32)>,
}

impl Default for UiConfig {
    fn default() -> Self {
        Self {
            theme: Theme::System,
            font_size: FontSize::Medium,
            show_timestamps: true,
            show_message_status: true,
            sound_notifications: true,
            notification_sound: "default".to_string(),
            auto_scroll: true,
            message_grouping: true,
            compact_mode: false,
            window_size: (800, 600),
            window_position: None,
        }
    }
}

/// Theme options
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Theme {
    Light,
    Dark,
    System,
}

/// Font size options
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum FontSize {
    Small,
    Medium,
    Large,
    ExtraLarge,
}

/// Storage configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfig {
    pub data_directory: PathBuf,
    pub max_messages: usize,
    pub message_retention_days: u32,
    pub enable_compression: bool,
    pub backup_enabled: bool,
    pub backup_interval: u64, // hours
    pub max_backup_files: u32,
}

impl Default for StorageConfig {
    fn default() -> Self {
        let mut data_dir = dirs::data_dir().unwrap_or_else(|| PathBuf::from("."));
        data_dir.push("tcp-messenger");

        Self {
            data_directory: data_dir,
            max_messages: 10000,
            message_retention_days: 30,
            enable_compression: true,
            backup_enabled: true,
            backup_interval: 24,
            max_backup_files: 7,
        }
    }
}

/// Logging configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    pub level: LogLevel,
    pub file_logging: bool,
    pub console_logging: bool,
    pub log_file: PathBuf,
    pub max_log_size: u64, // bytes
    pub max_log_files: u32,
    pub log_format: LogFormat,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        let mut log_file = dirs::data_dir().unwrap_or_else(|| PathBuf::from("."));
        log_file.push("tcp-messenger");
        log_file.push("logs");
        log_file.push("app.log");

        Self {
            level: LogLevel::Info,
            file_logging: true,
            console_logging: true,
            log_file,
            max_log_size: 10 * 1024 * 1024, // 10MB
            max_log_files: 5,
            log_format: LogFormat::Json,
        }
    }
}

/// Log levels
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

/// Log formats
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum LogFormat {
    Json,
    Text,
    Compact,
}

impl AppConfig {
    /// Load configuration from file
    pub fn load_from_file(path: &PathBuf) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }

        let content = std::fs::read_to_string(path)
            .map_err(|e| MessengerError::Config(format!("Failed to read config file: {}", e)))?;

        let config: AppConfig = serde_json::from_str(&content)
            .map_err(|e| MessengerError::Config(format!("Failed to parse config file: {}", e)))?;

        Ok(config)
    }

    /// Save configuration to file
    pub fn save_to_file(&self, path: &PathBuf) -> Result<()> {
        // Create directory if it doesn't exist
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| MessengerError::Config(format!("Failed to create config directory: {}", e)))?;
        }

        let content = serde_json::to_string_pretty(self)
            .map_err(|e| MessengerError::Config(format!("Failed to serialize config: {}", e)))?;

        std::fs::write(path, content)
            .map_err(|e| MessengerError::Config(format!("Failed to write config file: {}", e)))?;

        Ok(())
    }

    /// Validate configuration
    pub fn validate(&self) -> Result<()> {
        // Validate port range
        if self.network.server.port_range.0 >= self.network.server.port_range.1 {
            return Err(MessengerError::Config("Invalid port range".to_string()));
        }

        // Validate max clients
        if self.network.server.max_clients == 0 {
            return Err(MessengerError::Config("Max clients must be greater than 0".to_string()));
        }

        // Validate message size
        if self.security.max_message_size == 0 {
            return Err(MessengerError::Config("Max message size must be greater than 0".to_string()));
        }

        // Validate file size
        if self.security.max_file_size == 0 {
            return Err(MessengerError::Config("Max file size must be greater than 0".to_string()));
        }

        // Validate retention days
        if self.storage.message_retention_days == 0 {
            return Err(MessengerError::Config("Message retention days must be greater than 0".to_string()));
        }

        Ok(())
    }

    /// Get the default config file path
    pub fn default_config_path() -> PathBuf {
        let mut path = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
        path.push("tcp-messenger");
        path.push("config.json");
        path
    }

    /// Check if a file type is allowed
    pub fn is_file_type_allowed(&self, file_path: &str) -> bool {
        if let Some(extension) = std::path::Path::new(file_path).extension() {
            let ext = format!(".{}", extension.to_string_lossy().to_lowercase());
            self.security.allowed_file_types.contains(&ext)
        } else {
            false
        }
    }

    /// Get the next available port in the range
    pub fn get_next_available_port(&self) -> Result<u16> {
        use std::net::{IpAddr, Ipv4Addr, SocketAddr, TcpListener};

        let start_port = self.network.server.port_range.0;
        let end_port = self.network.server.port_range.1;

        for port in start_port..=end_port {
            let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), port);
            if TcpListener::bind(addr).is_ok() {
                return Ok(port);
            }
        }

        Err(MessengerError::Config("No available ports in range".to_string()))
    }
}
