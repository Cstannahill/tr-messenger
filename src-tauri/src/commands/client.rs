use crate::error::Result;
use crate::types::ClientInfo;
use crate::AppState;
use tauri::State;
use tracing::{info, error};

/// Connect to a TCP server
#[tauri::command]
pub async fn connect_to_server(
    address: String,
    port: u16,
    state: State<'_, AppState>,
) -> Result<ClientInfo> {
    info!("Connecting to server at {}:{}", address, port);

    let mut network_manager = state.network_manager.write().await;
    
    if network_manager.is_some() {
        return Err(crate::error::MessengerError::AlreadyConnected);
    }

    // Create new network manager and connect to server
    let (mut manager, _message_sender) = crate::network::NetworkManager::new();
    let client_info = manager.connect_to_server(address.clone(), port).await?;
    
    // Store the network manager in state
    *network_manager = Some(manager);
    
    info!("Connected to server at {}:{}", address, port);
    Ok(client_info)
}

/// Disconnect from the server
#[tauri::command]
pub async fn disconnect(state: State<'_, AppState>) -> Result<()> {
    info!("Disconnecting from server");

    let mut network_manager = state.network_manager.write().await;
    
    if let Some(mut manager) = network_manager.take() {
        manager.disconnect().await?;
        info!("Disconnected from server successfully");
    } else {
        return Err(crate::error::MessengerError::NotConnected);
    }
    
    Ok(())
}

/// Get connection status
#[tauri::command]
pub async fn get_connection_status(state: State<'_, AppState>) -> Result<Option<ClientInfo>> {
    let network_manager = state.network_manager.read().await;
    
    if let Some(manager) = network_manager.as_ref() {
        Ok(manager.client_info.clone())
    } else {
        Ok(None)
    }
}

/// Get client information
#[tauri::command]
pub fn get_client_info(_state: State<'_, AppState>) -> Result<Option<ClientInfo>> {
    // For now, return None
    // TODO: Implement actual client info retrieval
    Ok(None)
}

/// Check if client is connected
#[tauri::command]
pub fn is_connected(_state: State<'_, AppState>) -> Result<bool> {
    // For now, return false
    // TODO: Implement actual connection check
    Ok(false)
}

/// Get connection statistics
#[tauri::command]
pub fn get_connection_stats(_state: State<'_, AppState>) -> Result<crate::types::NetworkStats> {
    // For now, return default stats
    // TODO: Implement actual stats retrieval
    Ok(crate::types::NetworkStats::default())
}

/// Test connection to a server
#[tauri::command]
pub fn test_connection(address: String, port: u16) -> Result<bool> {
    info!("Testing connection to {}:{}", address, port);
    
    match std::net::TcpStream::connect(format!("{}:{}", address, port)) {
        Ok(_) => {
            info!("Connection test successful");
            Ok(true)
        },
        Err(e) => {
            error!("Connection test failed: {}", e);
            Ok(false)
        }
    }
}

/// Get client configuration
#[tauri::command]
pub fn get_client_config(_state: State<'_, AppState>) -> Result<crate::config::ClientConfig> {
    // For now, return default config
    // TODO: Implement actual config retrieval
    Ok(crate::config::ClientConfig::default())
}

/// Update client configuration
#[tauri::command]
pub fn update_client_config(
    _new_config: crate::config::ClientConfig,
    _state: State<'_, AppState>,
) -> Result<()> {
    // For now, just return success
    // TODO: Implement actual config update
    Ok(())
}

/// Get network discovery configuration
#[tauri::command]
pub fn get_discovery_config(_state: State<'_, AppState>) -> Result<crate::config::DiscoveryConfig> {
    // For now, return default config
    // TODO: Implement actual config retrieval
    Ok(crate::config::DiscoveryConfig::default())
}

/// Update network discovery configuration
#[tauri::command]
pub fn update_discovery_config(
    _new_config: crate::config::DiscoveryConfig,
    _state: State<'_, AppState>,
) -> Result<()> {
    // For now, just return success
    // TODO: Implement actual config update
    Ok(())
}

// Discovery functionality moved to discovery module
