use crate::error::Result;
use crate::types::ServerInfo;
use crate::AppState;
use tauri::State;
use tracing::info;

/// Start a TCP server
#[tauri::command]
pub async fn start_server(
    port: Option<u16>,
    state: State<'_, AppState>,
) -> Result<ServerInfo> {
    info!("Starting TCP server on port {:?}", port);

    let mut network_manager = state.network_manager.write().await;
    
    if network_manager.is_some() {
        return Err(crate::error::MessengerError::AlreadyConnected);
    }

    // Create new network manager and start server
    let (mut manager, _message_sender) = crate::network::NetworkManager::new();
    let server_info = manager.start_server(port).await?;
    
    // Store the network manager in state
    *network_manager = Some(manager);
    
    info!("TCP server started successfully on port {}", server_info.port);
    Ok(server_info)
}

/// Stop the TCP server
#[tauri::command]
pub async fn stop_server(state: State<'_, AppState>) -> Result<()> {
    info!("Stopping TCP server");

    let mut network_manager = state.network_manager.write().await;
    
    if let Some(mut manager) = network_manager.take() {
        manager.stop_server().await?;
        info!("TCP server stopped successfully");
    } else {
        return Err(crate::error::MessengerError::NotConnected);
    }
    
    Ok(())
}

/// Get server status
#[tauri::command]
pub async fn get_server_status(state: State<'_, AppState>) -> Result<Option<ServerInfo>> {
    let network_manager = state.network_manager.read().await;
    
    if let Some(manager) = network_manager.as_ref() {
        Ok(manager.server_info.clone())
    } else {
        Ok(None)
    }
}

/// Get server statistics
#[tauri::command]
pub fn get_server_stats(_state: State<'_, AppState>) -> Result<crate::types::NetworkStats> {
    // For now, return default stats
    // TODO: Implement actual server stats functionality
    Ok(crate::types::NetworkStats::default())
}

/// Check if server is running
#[tauri::command]
pub fn is_server_running(_state: State<'_, AppState>) -> Result<bool> {
    // For now, return false
    // TODO: Implement actual server running check
    Ok(false)
}

/// Get available ports in the configured range
#[tauri::command]
pub fn get_available_ports(_state: State<'_, AppState>) -> Result<Vec<u16>> {
    // For now, return a few common ports
    // TODO: Implement actual port scanning
    Ok(vec![8000, 8001, 8002, 8003, 8004])
}

/// Get server configuration
#[tauri::command]
pub fn get_server_config(_state: State<'_, AppState>) -> Result<crate::config::ServerConfig> {
    // For now, return default config
    // TODO: Implement actual config retrieval
    Ok(crate::config::ServerConfig::default())
}

/// Update server configuration
#[tauri::command]
pub fn update_server_config(
    _new_config: crate::config::ServerConfig,
    _state: State<'_, AppState>,
) -> Result<()> {
    // For now, just return success
    // TODO: Implement actual config update
    Ok(())
}
