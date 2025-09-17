use crate::error::Result;
use crate::discovery::{NetworkDiscovery, DiscoveredServer};
use crate::AppState;
use tauri::State;
use tracing::{info, debug};

/// Discover servers on the local network
#[tauri::command]
pub async fn discover_servers(
    _state: State<'_, AppState>,
) -> Result<Vec<DiscoveredServer>> {
    info!("Starting server discovery");

    let mut discovery = NetworkDiscovery::default();
    let servers = discovery.discover_servers().await?;
    
    info!("Found {} servers", servers.len());
    Ok(servers)
}

/// Get discovered servers (cached)
#[tauri::command]
pub async fn get_discovered_servers(
    _state: State<'_, AppState>,
) -> Result<Vec<DiscoveredServer>> {
    // For now, return empty list
    // TODO: Implement caching of discovered servers
    debug!("Getting discovered servers (cached)");
    Ok(Vec::new())
}

/// Start server announcement
#[tauri::command]
pub async fn start_server_announcement(
    server_id: String,
    server_name: String,
    server_port: u16,
    _state: State<'_, AppState>,
) -> Result<()> {
    info!("Starting server announcement for {}", server_name);

    let server_uuid = server_id.parse()
        .map_err(|e| crate::error::MessengerError::InvalidInput(format!("Invalid server ID: {}", e)))?;

    let discovery = NetworkDiscovery::default();
    discovery.start_server_announcement(server_uuid, server_name, server_port).await?;
    
    info!("Server announcement started");
    Ok(())
}

/// Stop server announcement
#[tauri::command]
pub async fn stop_server_announcement(
    _state: State<'_, AppState>,
) -> Result<()> {
    info!("Stopping server announcement");
    
    // TODO: Implement actual stop functionality
    // For now, just return success
    
    info!("Server announcement stopped");
    Ok(())
}
