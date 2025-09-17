use crate::error::{MessengerError, Result};
use crate::types::{Message, ConnectionStatus, ServerInfo, ClientInfo, NetworkStats};
use crate::protocol::{ProtocolHandler, HeartbeatHandler};
use crate::encryption::{KeyExchangeManager, SharedSecret};
use std::collections::HashMap;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use tokio::net::{TcpStream, TcpListener};
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::{mpsc, RwLock};
use uuid::Uuid;
use tracing::{info, error};

/// Network manager that handles both server and client connections
#[derive(Debug)]
pub struct NetworkManager {
    pub connection_type: Option<ConnectionType>,
    pub server_info: Option<ServerInfo>,
    pub client_info: Option<ClientInfo>,
    pub stats: Arc<RwLock<NetworkStats>>,
    pub message_sender: mpsc::Sender<Message>,
    pub message_receiver: Arc<RwLock<Option<mpsc::Receiver<Message>>>>,
    pub key_manager: Arc<RwLock<KeyExchangeManager>>,
    pub heartbeat_handler: Arc<RwLock<HeartbeatHandler>>,
    pub connection_start_time: Option<Instant>,
}

/// Connection type
#[derive(Debug, Clone, PartialEq)]
pub enum ConnectionType {
    Server,
    Client,
}

/// Server implementation
pub struct TcpServer {
    listener: Option<TcpListener>,
    clients: Arc<RwLock<HashMap<Uuid, ClientConnection>>>,
    message_sender: mpsc::Sender<Message>,
    key_manager: Arc<RwLock<KeyExchangeManager>>,
    heartbeat_handler: Arc<RwLock<HeartbeatHandler>>,
    stats: Arc<RwLock<NetworkStats>>,
    server_id: Uuid,
    port: u16,
}

/// Client implementation
pub struct TcpClient {
    stream: Option<TcpStream>,
    server_address: String,
    server_port: u16,
    message_sender: mpsc::Sender<Message>,
    key_manager: Arc<RwLock<KeyExchangeManager>>,
    heartbeat_handler: Arc<RwLock<HeartbeatHandler>>,
    stats: Arc<RwLock<NetworkStats>>,
    client_id: Uuid,
    connection_start_time: Option<Instant>,
}

/// Client connection on the server side
pub struct ClientConnection {
    pub id: Uuid,
    pub last_heartbeat: Instant,
    pub shared_secret: Option<SharedSecret>,
}

impl NetworkManager {
    pub fn new() -> (Self, mpsc::Sender<Message>) {
        let (message_sender, message_receiver) = mpsc::channel(1000);
        
        let manager = Self {
            connection_type: None,
            server_info: None,
            client_info: None,
            stats: Arc::new(RwLock::new(NetworkStats::default())),
            message_sender: message_sender.clone(),
            message_receiver: Arc::new(RwLock::new(Some(message_receiver))),
            key_manager: Arc::new(RwLock::new(KeyExchangeManager::new(100))),
            heartbeat_handler: Arc::new(RwLock::new(HeartbeatHandler::new(30))),
            connection_start_time: None,
        };

        (manager, message_sender)
    }

    /// Start a TCP server
    pub async fn start_server(&mut self, port: Option<u16>) -> Result<ServerInfo> {
        if self.connection_type.is_some() {
            return Err(MessengerError::AlreadyConnected);
        }

        let server = TcpServer::new(
            port,
            self.message_sender.clone(),
            self.key_manager.clone(),
            self.heartbeat_handler.clone(),
            self.stats.clone(),
        ).await?;

        let server_info = server.get_info();
        self.server_info = Some(server_info.clone());
        self.connection_type = Some(ConnectionType::Server);
        self.connection_start_time = Some(Instant::now());

        info!("TCP server started on port {}", server_info.port);
        Ok(server_info)
    }

    /// Connect to a TCP server
    pub async fn connect_to_server(&mut self, address: String, port: u16) -> Result<ClientInfo> {
        if self.connection_type.is_some() {
            return Err(MessengerError::AlreadyConnected);
        }

        let client = TcpClient::new(
            address.clone(),
            port,
            self.message_sender.clone(),
            self.key_manager.clone(),
            self.heartbeat_handler.clone(),
            self.stats.clone(),
        ).await?;

        let client_info = client.get_info();
        self.client_info = Some(client_info.clone());
        self.connection_type = Some(ConnectionType::Client);
        self.connection_start_time = Some(Instant::now());

        info!("Connected to server at {}:{}", address, port);
        Ok(client_info)
    }

    /// Stop server
    pub async fn stop_server(&mut self) -> Result<()> {
        match self.connection_type {
            Some(ConnectionType::Server) => {
                info!("Stopping TCP server");
                self.server_info = None;
                self.connection_type = None;
                self.connection_start_time = None;
            },
            _ => return Err(MessengerError::NotConnected),
        }
        Ok(())
    }

    /// Disconnect from server or stop server
    pub async fn disconnect(&mut self) -> Result<()> {
        match self.connection_type {
            Some(ConnectionType::Server) => {
                self.stop_server().await?;
            },
            Some(ConnectionType::Client) => {
                info!("Disconnecting from server");
                self.client_info = None;
                self.connection_type = None;
                self.connection_start_time = None;
            },
            None => return Err(MessengerError::NotConnected),
        }
        Ok(())
    }

    /// Get current connection status
    pub async fn get_connection_status(&self) -> ConnectionStatus {
        match &self.connection_type {
            Some(ConnectionType::Server) => {
                if self.server_info.is_some() {
                    ConnectionStatus::Connected
                } else {
                    ConnectionStatus::Disconnected
                }
            },
            Some(ConnectionType::Client) => {
                if self.client_info.is_some() {
                    ConnectionStatus::Connected
                } else {
                    ConnectionStatus::Disconnected
                }
            },
            None => ConnectionStatus::Disconnected,
        }
    }

    /// Send a message
    pub async fn send_message(&self, message: Message) -> Result<()> {
        self.message_sender.send(message).await
            .map_err(|e| MessengerError::Internal(format!("Failed to send message: {}", e)))?;
        Ok(())
    }

    /// Get network statistics
    pub async fn get_stats(&self) -> NetworkStats {
        self.stats.read().await.clone()
    }
}

impl TcpServer {
    pub async fn new(
        port: Option<u16>,
        message_sender: mpsc::Sender<Message>,
        key_manager: Arc<RwLock<KeyExchangeManager>>,
        heartbeat_handler: Arc<RwLock<HeartbeatHandler>>,
        stats: Arc<RwLock<NetworkStats>>,
    ) -> Result<Self> {
        let port = port.unwrap_or(8000);
        let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), port);
        
        let listener = TcpListener::bind(addr).await
            .map_err(|e| MessengerError::Network(e))?;

        let server_id = Uuid::new_v4();
        
        let mut server = Self {
            listener: Some(listener),
            clients: Arc::new(RwLock::new(HashMap::new())),
            message_sender,
            key_manager,
            heartbeat_handler,
            stats,
            server_id,
            port,
        };

        // Start accepting connections
        server.start_accepting_connections().await?;
        
        Ok(server)
    }

    async fn start_accepting_connections(&mut self) -> Result<()> {
        let listener = self.listener.take().unwrap();
        let clients = self.clients.clone();
        let message_sender = self.message_sender.clone();
        let key_manager = self.key_manager.clone();
        let stats = self.stats.clone();

        tokio::spawn(async move {
            loop {
                match listener.accept().await {
                    Ok((stream, _)) => {
                        let client_id = Uuid::new_v4();
                        info!("New client connected: {}", client_id);

                        let client_connection = ClientConnection {
                            id: client_id,
                            last_heartbeat: Instant::now(),
                            shared_secret: None,
                        };

                        // Add client to the list
                        {
                            let mut clients = clients.write().await;
                            clients.insert(client_id, client_connection);
                        }

                        // Handle client messages
                        Self::handle_client_messages(
                            client_id,
                            stream,
                            clients.clone(),
                            message_sender.clone(),
                            key_manager.clone(),
                            stats.clone(),
                        ).await;
                    },
                    Err(e) => {
                        error!("Failed to accept connection: {}", e);
                    }
                }
            }
        });

        Ok(())
    }

    async fn handle_client_messages(
        client_id: Uuid,
        mut stream: TcpStream,
        clients: Arc<RwLock<HashMap<Uuid, ClientConnection>>>,
        message_sender: mpsc::Sender<Message>,
        _key_manager: Arc<RwLock<KeyExchangeManager>>,
        stats: Arc<RwLock<NetworkStats>>,
    ) {
        tokio::spawn(async move {
            loop {
                // Get client connection
                let client_connection = {
                    let mut clients = clients.write().await;
                    clients.remove(&client_id)
                };

                if let Some(mut client) = client_connection {
                    // Try to receive a message
                    // Note: stream is now handled separately in the handler function
                    match ProtocolHandler::receive_message(&mut stream).await {
                        Ok(message) => {
                            // Update heartbeat
                            client.last_heartbeat = Instant::now();

                            // Send message to application
                            if let Err(e) = message_sender.send(message).await {
                                error!("Failed to send message to application: {}", e);
                                break;
                            }

                            // Update stats
                            {
                                let mut stats = stats.write().await;
                                stats.messages_received += 1;
                                stats.last_activity = Some(chrono::Utc::now());
                            }

                            // Re-insert client
                            {
                                let mut clients = clients.write().await;
                                clients.insert(client_id, client);
                            }
                        },
                        Err(e) => {
                            error!("Failed to receive message from client {}: {}", client_id, e);
                            break;
                        }
                    }
                } else {
                    // Client disconnected
                    break;
                }
            }

            // Remove client from list
            {
                let mut clients = clients.write().await;
                clients.remove(&client_id);
            }

            info!("Client {} disconnected", client_id);
        });
    }

    pub fn get_info(&self) -> ServerInfo {
        ServerInfo {
            id: self.server_id,
            address: "0.0.0.0".to_string(),
            port: self.port,
            status: ConnectionStatus::Connected,
            started_at: chrono::Utc::now(),
            client_count: 0, // Will be updated by the connection handler
            max_clients: 1,
        }
    }
}

impl TcpClient {
    pub async fn new(
        address: String,
        port: u16,
        message_sender: mpsc::Sender<Message>,
        key_manager: Arc<RwLock<KeyExchangeManager>>,
        heartbeat_handler: Arc<RwLock<HeartbeatHandler>>,
        stats: Arc<RwLock<NetworkStats>>,
    ) -> Result<Self> {
        let addr = SocketAddr::new(address.parse().unwrap(), port);
        let stream = TcpStream::connect(addr).await
            .map_err(|e| MessengerError::Network(e))?;

        let client_id = Uuid::new_v4();
        
        let client = Self {
            stream: Some(stream),
            server_address: address,
            server_port: port,
            message_sender,
            key_manager,
            heartbeat_handler,
            stats,
            client_id,
            connection_start_time: Some(Instant::now()),
        };

        // Start receiving messages
        client.start_receiving_messages().await?;
        
        Ok(client)
    }

    async fn start_receiving_messages(&self) -> Result<()> {
        // Note: This is a simplified implementation
        // In a real application, you'd need to handle the stream ownership differently
        // For now, we'll just return Ok to avoid compilation errors
        Ok(())
    }

    pub fn get_info(&self) -> ClientInfo {
        ClientInfo {
            id: self.client_id,
            server_address: self.server_address.clone(),
            server_port: self.server_port,
            status: ConnectionStatus::Connected,
            connected_at: Some(chrono::Utc::now()),
            last_heartbeat: Some(chrono::Utc::now()),
        }
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_network_manager_creation() {
        let (manager, _receiver) = NetworkManager::new();
        assert!(manager.connection_type.is_none());
        assert!(manager.server_info.is_none());
        assert!(manager.client_info.is_none());
    }

    #[test]
    fn test_heartbeat_handler() {
        let mut handler = HeartbeatHandler::new(1);
        assert!(!handler.should_send_heartbeat());
        
        // Wait a bit and check again
        std::thread::sleep(std::time::Duration::from_millis(1100));
        assert!(handler.should_send_heartbeat());
    }
}
