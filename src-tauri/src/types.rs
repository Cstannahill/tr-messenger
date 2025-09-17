use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;
use chrono::{DateTime, Utc};

/// Message types that can be sent through the system
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", content = "data")]
pub enum MessageType {
    /// Plain text message
    Text { content: String },
    /// File transfer message
    File { 
        name: String, 
        size: u64, 
        mime_type: String,
        data: Option<Vec<u8>>, // Only included for small files
        chunk_index: Option<u32>,
        total_chunks: Option<u32>,
    },
    /// System message (connection status, errors, etc.)
    System { content: String, level: SystemMessageLevel },
    /// Heartbeat message to keep connection alive
    Heartbeat,
    /// Key exchange message for encryption
    KeyExchange { public_key: Vec<u8> },
    /// Disconnect notification
    Disconnect { reason: String },
    /// Message acknowledgment
    Acknowledgment { message_id: Uuid },
}

/// System message severity levels
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SystemMessageLevel {
    Info,
    Warning,
    Error,
    Success,
}

/// Message status tracking
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MessageStatus {
    Sending,
    Sent,
    Delivered,
    Failed,
    Acknowledged,
}

impl std::fmt::Display for MessageStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MessageStatus::Sending => write!(f, "Sending"),
            MessageStatus::Sent => write!(f, "Sent"),
            MessageStatus::Delivered => write!(f, "Delivered"),
            MessageStatus::Failed => write!(f, "Failed"),
            MessageStatus::Acknowledged => write!(f, "Acknowledged"),
        }
    }
}

/// Message flags for protocol handling
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageFlags {
    None = 0,
    Encrypted = 1,
    Compressed = 2,
    Chunked = 4,
    Acknowledgment = 8,
}

impl From<u8> for MessageFlags {
    fn from(value: u8) -> Self {
        match value {
            0 => MessageFlags::None,
            1 => MessageFlags::Encrypted,
            2 => MessageFlags::Compressed,
            4 => MessageFlags::Chunked,
            8 => MessageFlags::Acknowledgment,
            _ => MessageFlags::None,
        }
    }
}

impl From<MessageFlags> for u8 {
    fn from(flags: MessageFlags) -> Self {
        flags as u8
    }
}

/// Core message structure
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Message {
    pub id: Uuid,
    pub message_type: MessageType,
    pub timestamp: DateTime<Utc>,
    pub sender_id: Uuid,
    pub recipient_id: Option<Uuid>,
    pub status: MessageStatus,
    pub encrypted: bool,
    pub retry_count: u32,
    pub metadata: HashMap<String, String>,
}

impl Message {
    /// Create a new text message
    pub fn new_text(content: String, sender_id: Uuid) -> Self {
        Self {
            id: Uuid::new_v4(),
            message_type: MessageType::Text { content },
            timestamp: Utc::now(),
            sender_id,
            recipient_id: None,
            status: MessageStatus::Sending,
            encrypted: false,
            retry_count: 0,
            metadata: HashMap::new(),
        }
    }

    /// Create a new system message
    pub fn new_system(content: String, level: SystemMessageLevel, sender_id: Uuid) -> Self {
        Self {
            id: Uuid::new_v4(),
            message_type: MessageType::System { content, level },
            timestamp: Utc::now(),
            sender_id,
            recipient_id: None,
            status: MessageStatus::Sent,
            encrypted: false,
            retry_count: 0,
            metadata: HashMap::new(),
        }
    }

    /// Create a new file message
    pub fn new_file(
        name: String,
        size: u64,
        mime_type: String,
        data: Option<Vec<u8>>,
        sender_id: Uuid,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            message_type: MessageType::File {
                name,
                size,
                mime_type,
                data,
                chunk_index: None,
                total_chunks: None,
            },
            timestamp: Utc::now(),
            sender_id,
            recipient_id: None,
            status: MessageStatus::Sending,
            encrypted: false,
            retry_count: 0,
            metadata: HashMap::new(),
        }
    }

    /// Create a heartbeat message
    pub fn new_heartbeat(sender_id: Uuid) -> Self {
        Self {
            id: Uuid::new_v4(),
            message_type: MessageType::Heartbeat,
            timestamp: Utc::now(),
            sender_id,
            recipient_id: None,
            status: MessageStatus::Sent,
            encrypted: false,
            retry_count: 0,
            metadata: HashMap::new(),
        }
    }

    /// Get the content size estimate for the message
    pub fn size_estimate(&self) -> usize {
        match &self.message_type {
            MessageType::Text { content } => content.len(),
            MessageType::File { data, .. } => data.as_ref().map_or(0, |d| d.len()),
            MessageType::System { content, .. } => content.len(),
            MessageType::Heartbeat => 0,
            MessageType::KeyExchange { public_key } => public_key.len(),
            MessageType::Disconnect { reason } => reason.len(),
            MessageType::Acknowledgment { .. } => 16, // UUID size
        }
    }

    /// Check if the message is a system message
    pub fn is_system(&self) -> bool {
        matches!(self.message_type, MessageType::System { .. })
    }

    /// Check if the message is a file transfer
    pub fn is_file(&self) -> bool {
        matches!(self.message_type, MessageType::File { .. })
    }
}

/// Connection types for the application
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ConnectionType {
    Server,
    Client,
}

/// Connection status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ConnectionStatus {
    Disconnected,
    Connecting,
    Connected,
    Reconnecting,
    Error(String),
}

/// Server information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerInfo {
    pub id: Uuid,
    pub address: String,
    pub port: u16,
    pub status: ConnectionStatus,
    pub started_at: DateTime<Utc>,
    pub client_count: u32,
    pub max_clients: u32,
}

/// Client connection information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientInfo {
    pub id: Uuid,
    pub server_address: String,
    pub server_port: u16,
    pub status: ConnectionStatus,
    pub connected_at: Option<DateTime<Utc>>,
    pub last_heartbeat: Option<DateTime<Utc>>,
}

/// Network statistics
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct NetworkStats {
    pub messages_sent: u64,
    pub messages_received: u64,
    pub bytes_sent: u64,
    pub bytes_received: u64,
    pub connection_uptime: u64, // in seconds
    pub last_activity: Option<DateTime<Utc>>,
}

/// File transfer information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileTransferInfo {
    pub id: Uuid,
    pub name: String,
    pub size: u64,
    pub mime_type: String,
    pub progress: f32, // 0.0 to 1.0
    pub status: FileTransferStatus,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub error: Option<String>,
}

/// File transfer status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum FileTransferStatus {
    Pending,
    InProgress,
    Completed,
    Failed,
    Cancelled,
}

/// User information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserInfo {
    pub id: Uuid,
    pub name: String,
    pub device_name: String,
    pub last_seen: DateTime<Utc>,
    pub is_online: bool,
}

/// Application state information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppInfo {
    pub version: String,
    pub build_date: String,
    pub platform: String,
    pub user_info: UserInfo,
    pub network_stats: NetworkStats,
    pub connection_type: Option<ConnectionType>,
    pub server_info: Option<ServerInfo>,
    pub client_info: Option<ClientInfo>,
}

/// Message filter for querying messages
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MessageFilter {
    pub message_types: Option<Vec<MessageType>>,
    pub sender_ids: Option<Vec<Uuid>>,
    pub start_date: Option<DateTime<Utc>>,
    pub end_date: Option<DateTime<Utc>>,
    pub status: Option<Vec<MessageStatus>>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

/// Search query for messages
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageSearch {
    pub query: String,
    pub case_sensitive: bool,
    pub search_content: bool,
    pub search_metadata: bool,
    pub filter: Option<MessageFilter>,
}

/// Export format for messages
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ExportFormat {
    Json,
    Csv,
    Txt,
    Html,
}

/// Export options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportOptions {
    pub format: ExportFormat,
    pub include_metadata: bool,
    pub include_system_messages: bool,
    pub date_range: Option<(DateTime<Utc>, DateTime<Utc>)>,
    pub filter: Option<MessageFilter>,
}
