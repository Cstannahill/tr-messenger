use crate::error::{MessengerError, Result};
use crate::types::{Message, MessageFilter, MessageSearch, ExportFormat, ExportOptions};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use uuid::Uuid;
use chrono::{DateTime, Utc};
use tracing::{info, debug};

/// Message storage implementation
#[derive(Debug, Default)]
pub struct MessageStorage {
    storage_path: PathBuf,
    messages: HashMap<Uuid, Message>,
    max_messages: usize,
    compression_enabled: bool,
}

/// Storage configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfig {
    pub data_directory: PathBuf,
    pub max_messages: usize,
    pub message_retention_days: u32,
    pub enable_compression: bool,
    pub backup_enabled: bool,
    pub backup_interval_hours: u64,
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
            backup_interval_hours: 24,
            max_backup_files: 7,
        }
    }
}

/// Message index for fast searching
#[derive(Debug, Clone, Serialize, Deserialize)]
struct MessageIndex {
    by_sender: HashMap<Uuid, Vec<Uuid>>,
    by_timestamp: Vec<Uuid>,
    by_type: HashMap<String, Vec<Uuid>>,
    by_content: HashMap<String, Vec<Uuid>>, // Simple keyword index
}

impl MessageStorage {
    /// Create a new message storage
    pub fn new() -> Self {
        let mut storage_path = dirs::data_dir().unwrap_or_else(|| PathBuf::from("."));
        storage_path.push("tcp-messenger");
        storage_path.push("messages");

        Self {
            storage_path,
            messages: HashMap::new(),
            max_messages: 10000,
            compression_enabled: true,
        }
    }

    /// Create message storage with custom configuration
    pub fn with_config(config: &StorageConfig) -> Self {
        let mut storage_path = config.data_directory.clone();
        storage_path.push("messages");

        Self {
            storage_path,
            messages: HashMap::new(),
            max_messages: config.max_messages,
            compression_enabled: config.enable_compression,
        }
    }

    /// Initialize storage (create directories, load existing messages)
    pub async fn initialize(&mut self) -> Result<()> {
        // Create storage directory if it doesn't exist
        std::fs::create_dir_all(&self.storage_path)
            .map_err(|e| MessengerError::Storage(format!("Failed to create storage directory: {}", e)))?;

        // Load existing messages
        self.load_messages().await?;

        info!("Message storage initialized with {} messages", self.messages.len());
        Ok(())
    }

    /// Store a message
    pub async fn store_message(&mut self, message: Message) -> Result<()> {
        let message_id = message.id;
        
        // Check if we need to remove old messages
        if self.messages.len() >= self.max_messages {
            self.cleanup_old_messages().await?;
        }

        // Store the message
        self.messages.insert(message_id, message.clone());

        // Persist to disk
        self.persist_message(&message).await?;

        debug!("Stored message: {}", message_id);
        Ok(())
    }

    /// Get a message by ID
    pub fn get_message(&self, message_id: &Uuid) -> Option<&Message> {
        self.messages.get(message_id)
    }

    /// Get all messages
    pub fn get_all_messages(&self) -> Vec<&Message> {
        self.messages.values().collect()
    }

    /// Get messages with filter
    pub fn get_messages_with_filter(&self, filter: &MessageFilter) -> Vec<&Message> {
        let mut messages: Vec<&Message> = self.messages.values().collect();

        // Apply filters
        if let Some(message_types) = &filter.message_types {
            messages.retain(|msg| {
                match (&msg.message_type, message_types) {
                    (crate::types::MessageType::Text { .. }, types) => {
                        types.iter().any(|t| matches!(t, crate::types::MessageType::Text { .. }))
                    },
                    (crate::types::MessageType::File { .. }, types) => {
                        types.iter().any(|t| matches!(t, crate::types::MessageType::File { .. }))
                    },
                    (crate::types::MessageType::System { .. }, types) => {
                        types.iter().any(|t| matches!(t, crate::types::MessageType::System { .. }))
                    },
                    _ => false,
                }
            });
        }

        if let Some(sender_ids) = &filter.sender_ids {
            messages.retain(|msg| sender_ids.contains(&msg.sender_id));
        }

        if let Some(start_date) = &filter.start_date {
            messages.retain(|msg| msg.timestamp >= *start_date);
        }

        if let Some(end_date) = &filter.end_date {
            messages.retain(|msg| msg.timestamp <= *end_date);
        }

        if let Some(status) = &filter.status {
            messages.retain(|msg| status.contains(&msg.status));
        }

        // Apply pagination
        if let Some(offset) = filter.offset {
            messages = messages.into_iter().skip(offset).collect();
        }

        if let Some(limit) = filter.limit {
            messages = messages.into_iter().take(limit).collect();
        }

        // Sort by timestamp (newest first)
        messages.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));

        messages
    }

    /// Search messages
    pub fn search_messages(&self, search: &MessageSearch) -> Vec<&Message> {
        let mut results = Vec::new();

        for message in self.messages.values() {
            let mut matches = false;

            if search.search_content {
                match &message.message_type {
                    crate::types::MessageType::Text { content } => {
                        if search.case_sensitive {
                            matches = content.contains(&search.query);
                        } else {
                            matches = content.to_lowercase().contains(&search.query.to_lowercase());
                        }
                    },
                    crate::types::MessageType::System { content, .. } => {
                        if search.case_sensitive {
                            matches = content.contains(&search.query);
                        } else {
                            matches = content.to_lowercase().contains(&search.query.to_lowercase());
                        }
                    },
                    _ => {}
                }
            }

            if search.search_metadata {
                for (key, value) in &message.metadata {
                    if search.case_sensitive {
                        matches = matches || key.contains(&search.query) || value.contains(&search.query);
                    } else {
                        let query_lower = search.query.to_lowercase();
                        matches = matches || 
                            key.to_lowercase().contains(&query_lower) || 
                            value.to_lowercase().contains(&query_lower);
                    }
                }
            }

            if matches {
                results.push(message);
            }
        }

        // Apply additional filter if provided
        if let Some(filter) = &search.filter {
            let filtered_results = self.get_messages_with_filter(filter);
            let filtered_ids: std::collections::HashSet<_> = filtered_results.iter().map(|msg| msg.id).collect();
            results.retain(|msg| filtered_ids.contains(&msg.id));
        }

        // Sort by timestamp (newest first)
        results.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));

        results
    }

    /// Delete a message
    pub async fn delete_message(&mut self, message_id: &Uuid) -> Result<()> {
        if let Some(message) = self.messages.remove(message_id) {
            // Remove from disk
            self.remove_message_from_disk(&message).await?;
            debug!("Deleted message: {}", message_id);
        }
        Ok(())
    }

    /// Clear all messages
    pub async fn clear_all_messages(&mut self) -> Result<()> {
        self.messages.clear();
        
        // Clear disk storage
        if self.storage_path.exists() {
            std::fs::remove_dir_all(&self.storage_path)
                .map_err(|e| MessengerError::Storage(format!("Failed to clear storage: {}", e)))?;
        }

        info!("Cleared all messages");
        Ok(())
    }

    /// Export messages to file
    pub async fn export_messages(&self, options: &ExportOptions) -> Result<PathBuf> {
        let messages = if let Some(filter) = &options.filter {
            self.get_messages_with_filter(filter)
        } else {
            self.get_all_messages()
        };

        let export_path = self.get_export_path(&options.format).await?;

        match options.format {
            ExportFormat::Json => self.export_to_json(&messages, &export_path).await?,
            ExportFormat::Csv => self.export_to_csv(&messages, &export_path).await?,
            ExportFormat::Txt => self.export_to_txt(&messages, &export_path).await?,
            ExportFormat::Html => self.export_to_html(&messages, &export_path).await?,
        }

        info!("Exported {} messages to {:?}", messages.len(), export_path);
        Ok(export_path)
    }

    /// Get storage statistics
    pub fn get_stats(&self) -> StorageStats {
        StorageStats {
            total_messages: self.messages.len(),
            storage_size_bytes: self.calculate_storage_size(),
            oldest_message: self.get_oldest_message_timestamp(),
            newest_message: self.get_newest_message_timestamp(),
        }
    }

    // Private helper methods

    async fn load_messages(&mut self) -> Result<()> {
        let messages_file = self.storage_path.join("messages.json");
        
        if !messages_file.exists() {
            return Ok(());
        }

        let content = std::fs::read_to_string(&messages_file)
            .map_err(|e| MessengerError::Storage(format!("Failed to read messages file: {}", e)))?;

        let messages: Vec<Message> = serde_json::from_str(&content)
            .map_err(|e| MessengerError::Storage(format!("Failed to parse messages: {}", e)))?;

        for message in messages {
            self.messages.insert(message.id, message);
        }

        Ok(())
    }

    async fn persist_message(&self, message: &Message) -> Result<()> {
        let messages_file = self.storage_path.join("messages.json");
        
        // Read existing messages
        let mut all_messages = if messages_file.exists() {
            let content = std::fs::read_to_string(&messages_file)
                .map_err(|e| MessengerError::Storage(format!("Failed to read messages file: {}", e)))?;
            serde_json::from_str::<Vec<Message>>(&content)
                .map_err(|e| MessengerError::Storage(format!("Failed to parse messages: {}", e)))?
        } else {
            Vec::new()
        };

        // Add or update the message
        if let Some(existing_index) = all_messages.iter().position(|m| m.id == message.id) {
            all_messages[existing_index] = message.clone();
        } else {
            all_messages.push(message.clone());
        }

        // Write back to file
        let content = serde_json::to_string_pretty(&all_messages)
            .map_err(|e| MessengerError::Storage(format!("Failed to serialize messages: {}", e)))?;

        std::fs::write(&messages_file, content)
            .map_err(|e| MessengerError::Storage(format!("Failed to write messages file: {}", e)))?;

        Ok(())
    }

    async fn remove_message_from_disk(&self, message: &Message) -> Result<()> {
        let messages_file = self.storage_path.join("messages.json");
        
        if !messages_file.exists() {
            return Ok(());
        }

        let content = std::fs::read_to_string(&messages_file)
            .map_err(|e| MessengerError::Storage(format!("Failed to read messages file: {}", e)))?;

        let mut all_messages: Vec<Message> = serde_json::from_str(&content)
            .map_err(|e| MessengerError::Storage(format!("Failed to parse messages: {}", e)))?;

        // Remove the message
        all_messages.retain(|m| m.id != message.id);

        // Write back to file
        let content = serde_json::to_string_pretty(&all_messages)
            .map_err(|e| MessengerError::Storage(format!("Failed to serialize messages: {}", e)))?;

        std::fs::write(&messages_file, content)
            .map_err(|e| MessengerError::Storage(format!("Failed to write messages file: {}", e)))?;

        Ok(())
    }

    async fn cleanup_old_messages(&mut self) -> Result<()> {
        let cutoff_date = Utc::now() - chrono::Duration::days(30);
        
        let old_message_ids: Vec<Uuid> = self.messages
            .iter()
            .filter(|(_, msg)| msg.timestamp < cutoff_date)
            .map(|(id, _)| *id)
            .collect();

        let count = old_message_ids.len();
        for message_id in old_message_ids {
            if let Some(message) = self.messages.remove(&message_id) {
                self.remove_message_from_disk(&message).await?;
            }
        }

        info!("Cleaned up {} old messages", count);
        Ok(())
    }

    async fn get_export_path(&self, format: &ExportFormat) -> Result<PathBuf> {
        let mut export_path = self.storage_path.parent().unwrap().to_path_buf();
        export_path.push("exports");
        std::fs::create_dir_all(&export_path)
            .map_err(|e| MessengerError::Storage(format!("Failed to create export directory: {}", e)))?;

        let timestamp = Utc::now().format("%Y%m%d_%H%M%S");
        let extension = match format {
            ExportFormat::Json => "json",
            ExportFormat::Csv => "csv",
            ExportFormat::Txt => "txt",
            ExportFormat::Html => "html",
        };

        export_path.push(format!("messages_{}.{}", timestamp, extension));
        Ok(export_path)
    }

    async fn export_to_json(&self, messages: &[&Message], path: &Path) -> Result<()> {
        let file = File::create(path)
            .map_err(|e| MessengerError::Storage(format!("Failed to create export file: {}", e)))?;
        
        let mut writer = BufWriter::new(file);
        serde_json::to_writer_pretty(&mut writer, messages)
            .map_err(|e| MessengerError::Storage(format!("Failed to write JSON export: {}", e)))?;

        Ok(())
    }

    async fn export_to_csv(&self, messages: &[&Message], path: &Path) -> Result<()> {
        let file = File::create(path)
            .map_err(|e| MessengerError::Storage(format!("Failed to create export file: {}", e)))?;
        
        let mut writer = BufWriter::new(file);
        writer.write_all(b"id,timestamp,sender_id,type,content,status\n")
            .map_err(|e| MessengerError::Storage(format!("Failed to write CSV header: {}", e)))?;

        for message in messages {
            let content = match &message.message_type {
                crate::types::MessageType::Text { content } => content,
                crate::types::MessageType::System { content, .. } => content,
                _ => "",
            };

            writeln!(writer, "{},{},{},{:?},{},{:?}",
                message.id,
                message.timestamp.to_rfc3339(),
                message.sender_id,
                message.message_type,
                content.replace('\n', " ").replace('\r', " "),
                message.status
            ).map_err(|e| MessengerError::Storage(format!("Failed to write CSV row: {}", e)))?;
        }

        Ok(())
    }

    async fn export_to_txt(&self, messages: &[&Message], path: &Path) -> Result<()> {
        let file = File::create(path)
            .map_err(|e| MessengerError::Storage(format!("Failed to create export file: {}", e)))?;
        
        let mut writer = BufWriter::new(file);

        for message in messages {
            writeln!(writer, "[{}] {} ({})",
                message.timestamp.format("%Y-%m-%d %H:%M:%S"),
                message.sender_id,
                message.status
            ).map_err(|e| MessengerError::Storage(format!("Failed to write TXT header: {}", e)))?;

            match &message.message_type {
                crate::types::MessageType::Text { content } => {
                    writeln!(writer, "{}", content)
                        .map_err(|e| MessengerError::Storage(format!("Failed to write TXT content: {}", e)))?;
                },
                crate::types::MessageType::System { content, .. } => {
                    writeln!(writer, "[SYSTEM] {}", content)
                        .map_err(|e| MessengerError::Storage(format!("Failed to write TXT system message: {}", e)))?;
                },
                _ => {
                    writeln!(writer, "[{:?}]", message.message_type)
                        .map_err(|e| MessengerError::Storage(format!("Failed to write TXT message type: {}", e)))?;
                }
            }

            writeln!(writer, "").map_err(|e| MessengerError::Storage(format!("Failed to write TXT separator: {}", e)))?;
        }

        Ok(())
    }

    async fn export_to_html(&self, messages: &[&Message], path: &Path) -> Result<()> {
        let file = File::create(path)
            .map_err(|e| MessengerError::Storage(format!("Failed to create export file: {}", e)))?;
        
        let mut writer = BufWriter::new(file);

        writeln!(writer, r#"<!DOCTYPE html>
<html>
<head>
    <title>Message Export</title>
    <style>
        body {{ font-family: Arial, sans-serif; margin: 20px; }}
        .message {{ border: 1px solid #ccc; margin: 10px 0; padding: 10px; }}
        .header {{ font-weight: bold; color: #666; }}
        .content {{ margin-top: 5px; }}
    </style>
</head>
<body>
    <h1>Message Export</h1>
"#).map_err(|e| MessengerError::Storage(format!("Failed to write HTML header: {}", e)))?;

        for message in messages {
            writeln!(writer, r#"    <div class="message">
        <div class="header">[{}] {} ({})</div>
        <div class="content">"#,
                message.timestamp.format("%Y-%m-%d %H:%M:%S"),
                message.sender_id,
                message.status
            ).map_err(|e| MessengerError::Storage(format!("Failed to write HTML message header: {}", e)))?;

            match &message.message_type {
                crate::types::MessageType::Text { content } => {
                    writeln!(writer, "{}", html_escape(content))
                        .map_err(|e| MessengerError::Storage(format!("Failed to write HTML content: {}", e)))?;
                },
                crate::types::MessageType::System { content, .. } => {
                    writeln!(writer, r#"<em>[SYSTEM] {}</em>"#, html_escape(content))
                        .map_err(|e| MessengerError::Storage(format!("Failed to write HTML system message: {}", e)))?;
                },
                _ => {
                    writeln!(writer, r#"<em>[{:?}]</em>"#, message.message_type)
                        .map_err(|e| MessengerError::Storage(format!("Failed to write HTML message type: {}", e)))?;
                }
            }

            writeln!(writer, r#"        </div>
    </div>"#).map_err(|e| MessengerError::Storage(format!("Failed to write HTML message footer: {}", e)))?;
        }

        writeln!(writer, r#"</body>
</html>"#).map_err(|e| MessengerError::Storage(format!("Failed to write HTML footer: {}", e)))?;

        Ok(())
    }

    fn calculate_storage_size(&self) -> u64 {
        let mut total_size = 0;
        
        if let Ok(entries) = std::fs::read_dir(&self.storage_path) {
            for entry in entries.flatten() {
                if let Ok(metadata) = entry.metadata() {
                    total_size += metadata.len();
                }
            }
        }
        
        total_size
    }

    fn get_oldest_message_timestamp(&self) -> Option<DateTime<Utc>> {
        self.messages.values()
            .map(|msg| msg.timestamp)
            .min()
    }

    fn get_newest_message_timestamp(&self) -> Option<DateTime<Utc>> {
        self.messages.values()
            .map(|msg| msg.timestamp)
            .max()
    }
}

/// Storage statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageStats {
    pub total_messages: usize,
    pub storage_size_bytes: u64,
    pub oldest_message: Option<DateTime<Utc>>,
    pub newest_message: Option<DateTime<Utc>>,
}

impl Default for StorageStats {
    fn default() -> Self {
        Self {
            total_messages: 0,
            storage_size_bytes: 0,
            oldest_message: None,
            newest_message: None,
        }
    }
}

/// HTML escape function
fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#x27;")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::MessageType;

    #[tokio::test]
    async fn test_message_storage() {
        let mut storage = MessageStorage::new();
        storage.initialize().await.unwrap();

        let message = Message::new_text("Hello, World!".to_string(), Uuid::new_v4());
        storage.store_message(message.clone()).await.unwrap();

        let retrieved = storage.get_message(&message.id);
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().id, message.id);
    }

    #[tokio::test]
    async fn test_message_filtering() {
        let mut storage = MessageStorage::new();
        storage.initialize().await.unwrap();

        let sender_id = Uuid::new_v4();
        let message1 = Message::new_text("Message 1".to_string(), sender_id);
        let message2 = Message::new_text("Message 2".to_string(), Uuid::new_v4());

        storage.store_message(message1.clone()).await.unwrap();
        storage.store_message(message2.clone()).await.unwrap();

        let filter = MessageFilter {
            sender_ids: Some(vec![sender_id]),
            ..Default::default()
        };

        let filtered = storage.get_messages_with_filter(&filter);
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].id, message1.id);
    }
}
