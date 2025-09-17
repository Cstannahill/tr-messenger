// Core modules
pub mod config;
pub mod error;
pub mod types;
pub mod protocol;
pub mod encryption;
pub mod network;
pub mod storage;
pub mod discovery;
pub mod commands;

// Re-exports for easier access
pub use error::{MessengerError, Result};
pub use types::*;

use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, error};

// Application state
#[derive(Debug, Default)]
pub struct AppState {
    pub config: Arc<RwLock<config::AppConfig>>,
    pub network_manager: Arc<RwLock<Option<network::NetworkManager>>>,
    pub storage: Arc<RwLock<storage::MessageStorage>>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            config: Arc::new(RwLock::new(config::AppConfig::default())),
            network_manager: Arc::new(RwLock::new(None)),
            storage: Arc::new(RwLock::new(storage::MessageStorage::new())),
        }
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    info!("Starting TCP Messenger application");

    let app_state = AppState::new();

    tauri::Builder::default()
        .manage(app_state)
        .invoke_handler(tauri::generate_handler![
            commands::server::start_server,
            commands::server::stop_server,
            commands::server::get_server_status,
            commands::client::connect_to_server,
            commands::client::disconnect,
            commands::client::get_connection_status,
            commands::message::send_message,
            commands::message::get_messages,
            commands::message::send_file,
            commands::config::get_config,
            commands::config::update_config,
            commands::discovery::discover_servers,
            commands::discovery::get_discovered_servers,
            commands::discovery::start_server_announcement,
            commands::discovery::stop_server_announcement,
        ])
        .run(tauri::generate_context!())
        .unwrap_or_else(|e| {
            error!("Failed to run Tauri application: {}", e);
            std::process::exit(1);
        });
}
