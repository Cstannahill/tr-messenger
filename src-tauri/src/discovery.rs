use crate::error::{MessengerError, Result};
use std::net::{Ipv4Addr, SocketAddr, UdpSocket};
use std::time::{Duration, Instant};
use serde::{Deserialize, Serialize};
use tracing::{info, debug, warn};
use uuid::Uuid;

/// Discovery service for finding servers on the local network
pub struct NetworkDiscovery {
    broadcast_port: u16,
    service_name: String,
    timeout: Duration,
    socket: Option<UdpSocket>,
}

/// Discovery message sent over UDP
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveryMessage {
    pub message_type: DiscoveryMessageType,
    pub server_id: Uuid,
    pub server_name: String,
    pub server_port: u16,
    pub timestamp: u64,
}

/// Type of discovery message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DiscoveryMessageType {
    /// Server announcing its presence
    ServerAnnounce,
    /// Client requesting server list
    ClientRequest,
    /// Server responding to client request
    ServerResponse,
}

/// Discovered server information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveredServer {
    pub id: Uuid,
    pub name: String,
    pub address: String,
    pub port: u16,
    pub discovered_at: u64,  // Unix timestamp
    pub last_seen: u64,      // Unix timestamp
}

impl NetworkDiscovery {
    /// Create a new network discovery service
    pub fn new(broadcast_port: u16, service_name: String, timeout: Duration) -> Self {
        Self {
            broadcast_port,
            service_name,
            timeout,
            socket: None,
        }
    }

    /// Start the discovery service as a server
    pub async fn start_server_announcement(mut self, server_id: Uuid, server_name: String, server_port: u16) -> Result<()> {
        info!("Starting server discovery announcement on port {}", self.broadcast_port);

        let socket = UdpSocket::bind(format!("0.0.0.0:{}", self.broadcast_port))
            .map_err(|e| MessengerError::Network(e))?;
        
        socket.set_broadcast(true)
            .map_err(|e| MessengerError::Network(e))?;

        self.socket = Some(socket);
        let socket = self.socket.as_ref().unwrap();

        // Start announcement loop
        let announce_message = DiscoveryMessage {
            message_type: DiscoveryMessageType::ServerAnnounce,
            server_id,
            server_name: server_name.clone(),
            server_port,
            timestamp: chrono::Utc::now().timestamp() as u64,
        };

        let socket_clone = socket.try_clone()
            .map_err(|e| MessengerError::Network(e))?;
        let announce_message_clone = announce_message.clone();
        
        tokio::spawn(async move {
            loop {
                if let Err(e) = Self::broadcast_announcement(&socket_clone, &announce_message_clone).await {
                    warn!("Failed to broadcast announcement: {}", e);
                }
                
                tokio::time::sleep(Duration::from_secs(30)).await;
            }
        });

        info!("Server discovery announcement started");
        Ok(())
    }

    /// Discover servers on the local network
    pub async fn discover_servers(&mut self) -> Result<Vec<DiscoveredServer>> {
        info!("Starting server discovery");

        let socket = UdpSocket::bind("0.0.0.0:0")
            .map_err(|e| MessengerError::Network(e))?;
        
        socket.set_broadcast(true)
            .map_err(|e| MessengerError::Network(e))?;

        // Send discovery request
        let request_message = DiscoveryMessage {
            message_type: DiscoveryMessageType::ClientRequest,
            server_id: Uuid::new_v4(),
            server_name: "discovery-client".to_string(),
            server_port: 0,
            timestamp: chrono::Utc::now().timestamp() as u64,
        };

        let message_data = serde_json::to_vec(&request_message)
            .map_err(|e| MessengerError::Serialization(e))?;

        // Broadcast request to all interfaces
        let broadcast_addr = SocketAddr::from((Ipv4Addr::BROADCAST, self.broadcast_port));
        socket.send_to(&message_data, broadcast_addr)
            .map_err(|e| MessengerError::Network(e))?;

        debug!("Discovery request sent to {}", broadcast_addr);

        // Listen for responses
        let mut discovered_servers = Vec::new();
        let start_time = Instant::now();

        socket.set_read_timeout(Some(self.timeout))
            .map_err(|e| MessengerError::Network(e))?;

        while start_time.elapsed() < self.timeout {
            let mut buffer = [0u8; 1024];
            
            match socket.recv_from(&mut buffer) {
                Ok((size, addr)) => {
                    debug!("Received {} bytes from {}", size, addr);
                    
                    if let Ok(discovery_message) = serde_json::from_slice::<DiscoveryMessage>(&buffer[..size]) {
                        if matches!(discovery_message.message_type, DiscoveryMessageType::ServerResponse | DiscoveryMessageType::ServerAnnounce) {
                            let server_name = discovery_message.server_name.clone();
                            let server_port = discovery_message.server_port;
                            
                            let server = DiscoveredServer {
                                id: discovery_message.server_id,
                                name: server_name.clone(),
                                address: addr.ip().to_string(),
                                port: server_port,
                                discovered_at: chrono::Utc::now().timestamp() as u64,
                                last_seen: chrono::Utc::now().timestamp() as u64,
                            };
                            
                            // Avoid duplicates
                            if !discovered_servers.iter().any(|s: &DiscoveredServer| s.id == server.id) {
                                discovered_servers.push(server);
                                info!("Discovered server: {} at {}:{}", server_name, addr.ip(), server_port);
                            }
                        }
                    }
                }
                Err(e) if e.kind() == std::io::ErrorKind::TimedOut => {
                    break;
                }
                Err(e) => {
                    debug!("Error receiving discovery response: {}", e);
                }
            }
        }

        info!("Discovery completed, found {} servers", discovered_servers.len());
        Ok(discovered_servers)
    }

    /// Handle incoming discovery messages (for servers)
    async fn handle_discovery_messages(&self) -> Result<()> {
        if let Some(socket) = &self.socket {
            let mut buffer = [0u8; 1024];
            
            loop {
                match socket.recv_from(&mut buffer) {
                    Ok((size, addr)) => {
                        debug!("Received discovery message from {}", addr);
                        
                        if let Ok(discovery_message) = serde_json::from_slice::<DiscoveryMessage>(&buffer[..size]) {
                            match discovery_message.message_type {
                                DiscoveryMessageType::ClientRequest => {
                                    // Respond to client request
                                    let response = DiscoveryMessage {
                                        message_type: DiscoveryMessageType::ServerResponse,
                                        server_id: discovery_message.server_id, // Use our server ID
                                        server_name: "TCP Messenger Server".to_string(),
                                        server_port: discovery_message.server_port, // Use our server port
                                        timestamp: chrono::Utc::now().timestamp() as u64,
                                    };
                                    
                                    let response_data = serde_json::to_vec(&response)
                                        .map_err(|e| MessengerError::Serialization(e))?;
                                    
                                    socket.send_to(&response_data, addr)
                                        .map_err(|e| MessengerError::Network(e))?;
                                    
                                    debug!("Sent discovery response to {}", addr);
                                }
                                _ => {
                                    debug!("Ignoring discovery message type: {:?}", discovery_message.message_type);
                                }
                            }
                        }
                    }
                    Err(e) => {
                        debug!("Error receiving discovery message: {}", e);
                        tokio::time::sleep(Duration::from_millis(100)).await;
                    }
                }
            }
        }
        
        Ok(())
    }

    /// Broadcast server announcement
    async fn broadcast_announcement(socket: &UdpSocket, message: &DiscoveryMessage) -> Result<()> {
        let message_data = serde_json::to_vec(message)
            .map_err(|e| MessengerError::Serialization(e))?;

        let broadcast_addr = SocketAddr::from((Ipv4Addr::BROADCAST, 9000));
        socket.send_to(&message_data, broadcast_addr)
            .map_err(|e| MessengerError::Network(e))?;

        debug!("Broadcasted server announcement");
        Ok(())
    }

    /// Stop the discovery service
    pub fn stop(&mut self) {
        self.socket = None;
        info!("Network discovery service stopped");
    }
}

impl Default for NetworkDiscovery {
    fn default() -> Self {
        Self {
            broadcast_port: 9000,
            service_name: "tcp-messenger".to_string(),
            timeout: Duration::from_secs(5),
            socket: None,
        }
    }
}
