use crate::{protocol_error, error::{MessengerError, Result}};
use crate::types::Message;
use serde::{Deserialize, Serialize};
use tokio::net::TcpStream;

/// Protocol version
pub const PROTOCOL_VERSION: u8 = 1;

/// Message header structure (8 bytes)
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct MessageHeader {
    pub version: u8,
    pub message_type: u8,
    pub flags: u8,
    pub length: u32,
}


impl MessageHeader {
    pub fn new(message_type: u8, length: u32, flags: u8) -> Self {
        Self {
            version: PROTOCOL_VERSION,
            message_type,
            flags,
            length,
        }
    }

    /// Serialize header to bytes
    pub fn to_bytes(&self) -> [u8; 8] {
        let mut bytes = [0u8; 8];
        bytes[0] = self.version;
        bytes[1] = self.message_type;
        bytes[2] = self.flags;
        bytes[3] = 0; // Reserved
        bytes[4..8].copy_from_slice(&self.length.to_be_bytes());
        bytes
    }

    /// Deserialize header from bytes
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        if bytes.len() < 8 {
            return Err(protocol_error!("Invalid header length: {}", bytes.len()));
        }

        let version = bytes[0];
        if version != PROTOCOL_VERSION {
            return Err(protocol_error!("Unsupported protocol version: {}", version));
        }

        let message_type = bytes[1];
        let flags = bytes[2];
        let length = u32::from_be_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]);

        Ok(Self {
            version,
            message_type,
            flags,
            length,
        })
    }
}

/// Protocol message wrapper
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProtocolMessage {
    pub header: MessageHeader,
    pub data: Vec<u8>,
}

impl ProtocolMessage {
    pub fn new(message: &Message) -> Result<Self> {
        let serialized = serde_json::to_vec(message)
            .map_err(|e| protocol_error!("Failed to serialize message: {}", e))?;

        let message_type = match message.message_type {
            crate::types::MessageType::Text { .. } => 0x01,
            crate::types::MessageType::File { .. } => 0x02,
            crate::types::MessageType::System { .. } => 0x03,
            crate::types::MessageType::Heartbeat => 0x04,
            crate::types::MessageType::KeyExchange { .. } => 0x05,
            crate::types::MessageType::Disconnect { .. } => 0x06,
            crate::types::MessageType::Acknowledgment { .. } => 0x09,
        };

        let mut flags = 0u8;
        if message.encrypted { flags |= 0x01; }
        if !message.is_system() { flags |= 0x04; }

        let header = MessageHeader::new(message_type, serialized.len() as u32, flags);

        Ok(Self {
            header,
            data: serialized,
        })
    }

    /// Serialize the entire protocol message to bytes
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&self.header.to_bytes());
        bytes.extend_from_slice(&self.data);
        bytes
    }

    /// Deserialize protocol message from bytes
    pub fn from_bytes(data: &[u8]) -> Result<Self> {
        if data.len() < 8 {
            return Err(protocol_error!("Insufficient data for header"));
        }

        let header = MessageHeader::from_bytes(&data[0..8])?;
        
        if data.len() < 8 + header.length as usize {
            return Err(protocol_error!("Incomplete message data"));
        }

        let message_data = data[8..8 + header.length as usize].to_vec();

        Ok(Self {
            header,
            data: message_data,
        })
    }

    /// Convert back to application message
    pub fn to_message(&self) -> Result<Message> {
        let message: Message = serde_json::from_slice(&self.data)
            .map_err(|e| protocol_error!("Failed to deserialize message: {}", e))?;
        Ok(message)
    }
}

/// Protocol handler for reading/writing messages
pub struct ProtocolHandler;

impl ProtocolHandler {
    /// Send a message through a TCP stream
    pub async fn send_message(stream: &mut TcpStream, message: &Message) -> Result<()> {
        let protocol_msg = ProtocolMessage::new(message)?;
        let bytes = protocol_msg.to_bytes();
        
        use tokio::io::AsyncWriteExt;
        stream.write_all(&bytes).await
            .map_err(|e| protocol_error!("Failed to send message: {}", e))?;
        
        stream.flush().await
            .map_err(|e| protocol_error!("Failed to flush stream: {}", e))?;

        Ok(())
    }

    /// Receive a message from a TCP stream
    pub async fn receive_message(stream: &mut TcpStream) -> Result<Message> {
        use tokio::io::AsyncReadExt;
        
        // First, read the header (8 bytes)
        let mut header_bytes = [0u8; 8];
        stream.read_exact(&mut header_bytes).await
            .map_err(|e| protocol_error!("Failed to read header: {}", e))?;

        let header = MessageHeader::from_bytes(&header_bytes)?;

        // Then read the message data
        let mut data = vec![0u8; header.length as usize];
        stream.read_exact(&mut data).await
            .map_err(|e| protocol_error!("Failed to read message data: {}", e))?;

        let protocol_msg = ProtocolMessage { header, data };
        protocol_msg.to_message()
    }

    /// Send raw bytes (for encrypted data)
    pub async fn send_raw_bytes(stream: &mut TcpStream, data: &[u8]) -> Result<()> {
        use tokio::io::AsyncWriteExt;
        
        // Send length first (4 bytes)
        let length = data.len() as u32;
        stream.write_all(&length.to_be_bytes()).await
            .map_err(|e| protocol_error!("Failed to send length: {}", e))?;

        // Then send the data
        stream.write_all(data).await
            .map_err(|e| protocol_error!("Failed to send data: {}", e))?;

        stream.flush().await
            .map_err(|e| protocol_error!("Failed to flush stream: {}", e))?;

        Ok(())
    }

    /// Receive raw bytes (for encrypted data)
    pub async fn receive_raw_bytes(stream: &mut TcpStream) -> Result<Vec<u8>> {
        use tokio::io::AsyncReadExt;
        
        // First read the length (4 bytes)
        let mut length_bytes = [0u8; 4];
        stream.read_exact(&mut length_bytes).await
            .map_err(|e| protocol_error!("Failed to read length: {}", e))?;

        let length = u32::from_be_bytes(length_bytes) as usize;

        // Then read the data
        let mut data = vec![0u8; length];
        stream.read_exact(&mut data).await
            .map_err(|e| protocol_error!("Failed to read data: {}", e))?;

        Ok(data)
    }

    /// Check if the stream has data available
    pub async fn has_data_available(_stream: &TcpStream) -> Result<bool> {
        // For tokio::net::TcpStream, we can't easily check data availability
        // without potentially consuming data. This is a simplified implementation.
        // In a real application, you might want to use a different approach
        // like reading with a timeout or using a different method.
        Ok(true) // Assume data is available - let the actual read operation handle errors
    }
}

/// Message acknowledgment handler
pub struct AcknowledgmentHandler;

impl AcknowledgmentHandler {
    /// Create an acknowledgment message
    pub fn create_acknowledgment(message_id: uuid::Uuid, sender_id: uuid::Uuid) -> Message {
        Message {
            id: uuid::Uuid::new_v4(),
            message_type: crate::types::MessageType::Acknowledgment { message_id },
            timestamp: chrono::Utc::now(),
            sender_id,
            recipient_id: None,
            status: crate::types::MessageStatus::Sent,
            encrypted: false,
            retry_count: 0,
            metadata: std::collections::HashMap::new(),
        }
    }

    /// Check if a message requires acknowledgment
    pub fn requires_acknowledgment(message: &Message) -> bool {
        !message.is_system() && message.status != crate::types::MessageStatus::Acknowledged
    }
}

/// Heartbeat handler
#[derive(Debug)]
pub struct HeartbeatHandler {
    last_heartbeat: std::time::Instant,
    interval: std::time::Duration,
}

impl HeartbeatHandler {
    pub fn new(interval_seconds: u64) -> Self {
        Self {
            last_heartbeat: std::time::Instant::now(),
            interval: std::time::Duration::from_secs(interval_seconds),
        }
    }

    /// Check if it's time to send a heartbeat
    pub fn should_send_heartbeat(&self) -> bool {
        self.last_heartbeat.elapsed() >= self.interval
    }

    /// Update the last heartbeat time
    pub fn update_heartbeat(&mut self) {
        self.last_heartbeat = std::time::Instant::now();
    }

    /// Create a heartbeat message
    pub fn create_heartbeat(sender_id: uuid::Uuid) -> Message {
        Message::new_heartbeat(sender_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::MessageType;

    #[test]
    fn test_message_header_serialization() {
        let flags = MessageFlags::new();
        let header = MessageHeader::new(MessageType::Text, 100, flags);
        let bytes = header.to_bytes();
        let deserialized = MessageHeader::from_bytes(&bytes).unwrap();
        
        assert_eq!(header.version, deserialized.version);
        assert_eq!(header.message_type, deserialized.message_type);
        assert_eq!(header.length, deserialized.length);
    }

    #[test]
    fn test_message_flags() {
        let mut flags = MessageFlags::new();
        flags.encrypted = true;
        flags.acknowledgment_required = true;
        
        let byte = flags.to_byte();
        let deserialized = MessageFlags::from_byte(byte);
        
        assert_eq!(flags.encrypted, deserialized.encrypted);
        assert_eq!(flags.acknowledgment_required, deserialized.acknowledgment_required);
    }
}
