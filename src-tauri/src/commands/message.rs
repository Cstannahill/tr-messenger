use crate::error::Result;
use crate::types::{Message, MessageFilter, MessageSearch, ExportFormat};
use crate::AppState;
use tauri::State;
use tracing::{info, debug};
use uuid::Uuid;

/// Send a text message
#[tauri::command]
pub async fn send_message(
    content: String,
    state: State<'_, AppState>,
) -> Result<Uuid> {
    info!("Sending message: {}", content);

    let message = Message::new_text(content, Uuid::new_v4());
    let message_id = message.id;

    // Store message in storage
    {
        let mut storage = state.storage.write().await;
        storage.store_message(message.clone()).await?;
    }

    // Send message through network
    {
        let network_manager = state.network_manager.read().await;
        if let Some(manager) = network_manager.as_ref() {
            manager.send_message(message).await?;
        } else {
            return Err(crate::error::MessengerError::NotConnected);
        }
    }

    info!("Message sent successfully: {}", message_id);
    Ok(message_id)
}

/// Send a system message
#[tauri::command]
pub fn send_system_message(
    content: String,
    level: crate::types::SystemMessageLevel,
    state: State<'_, AppState>,
) -> Result<Uuid> {
    info!("Sending system message: {}", content);

    let message = Message::new_system(content, level, Uuid::new_v4());
    let message_id = message.id;

    // For now, just return the message ID
    // TODO: Implement actual message sending and storage
    info!("System message created successfully: {}", message_id);
    Ok(message_id)
}

/// Get all messages
#[tauri::command]
pub async fn get_messages(
    limit: Option<usize>,
    state: State<'_, AppState>,
) -> Result<Vec<Message>> {
    debug!("Getting messages with limit: {:?}", limit);

    let storage = state.storage.read().await;
    let messages: Vec<Message> = storage.get_all_messages().into_iter().cloned().collect();
    
    debug!("Retrieved {} messages", messages.len());
    Ok(messages)
}

/// Get messages with filter
#[tauri::command]
pub fn get_messages_with_filter(
    filter: MessageFilter,
    state: State<'_, AppState>,
) -> Result<Vec<Message>> {
    debug!("Getting messages with filter: {:?}", filter);

    // For now, return empty vector
    // TODO: Implement actual message filtering
    Ok(Vec::new())
}

/// Search messages
#[tauri::command]
pub fn search_messages(
    search: MessageSearch,
    state: State<'_, AppState>,
) -> Result<Vec<Message>> {
    debug!("Searching messages with query: {}", search.query);

    // For now, return empty vector
    // TODO: Implement actual message search
    Ok(Vec::new())
}

/// Get a specific message by ID
#[tauri::command]
pub fn get_message(
    message_id: Uuid,
    state: State<'_, AppState>,
) -> Result<Option<Message>> {
    debug!("Getting message: {}", message_id);

    // For now, return None
    // TODO: Implement actual message retrieval
    Ok(None)
}

/// Delete a message
#[tauri::command]
pub fn delete_message(
    message_id: Uuid,
    state: State<'_, AppState>,
) -> Result<()> {
    info!("Deleting message: {}", message_id);

    // For now, just return success
    // TODO: Implement actual message deletion
    info!("Message deleted successfully: {}", message_id);
    Ok(())
}

/// Clear all messages
#[tauri::command]
pub fn clear_all_messages(_state: State<'_, AppState>) -> Result<()> {
    info!("Clearing all messages");

    // For now, just return success
    // TODO: Implement actual message clearing
    info!("All messages cleared successfully");
    Ok(())
}

/// Send a file
#[tauri::command]
pub async fn send_file(
    file_path: String,
    state: State<'_, AppState>,
) -> Result<Uuid> {
    info!("Sending file: {}", file_path);

    // Read file metadata
    let metadata = std::fs::metadata(&file_path)
        .map_err(|e| crate::error::MessengerError::File(format!("Failed to read file metadata: {}", e)))?;

    let file_name = std::path::Path::new(&file_path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown")
        .to_string();

    let mime_type = mime_guess::from_path(&file_path)
        .first_or_octet_stream()
        .to_string();

    // For small files (< 1MB), send in single message
    let chunk_size = 1024 * 1024; // 1MB chunks
    if metadata.len() <= chunk_size as u64 {
        // Read entire file
        let file_data = std::fs::read(&file_path)
            .map_err(|e| crate::error::MessengerError::File(format!("Failed to read file: {}", e)))?;

        let message = Message::new_file(
            file_name,
            metadata.len(),
            mime_type,
            Some(file_data),
            Uuid::new_v4(),
        );

        let message_id = message.id;

        // Store and send message
        {
            let mut storage = state.storage.write().await;
            storage.store_message(message.clone()).await?;
        }

        {
            let network_manager = state.network_manager.read().await;
            if let Some(manager) = network_manager.as_ref() {
                manager.send_message(message).await?;
            } else {
                return Err(crate::error::MessengerError::NotConnected);
            }
        }

        info!("File sent in single message: {}", message_id);
        Ok(message_id)
    } else {
        // For large files, implement chunking
        let total_chunks = ((metadata.len() + chunk_size as u64 - 1) / chunk_size as u64) as u32;
        let file_id = Uuid::new_v4();

        info!("Sending large file in {} chunks", total_chunks);

        let mut file = std::fs::File::open(&file_path)
            .map_err(|e| crate::error::MessengerError::File(format!("Failed to open file: {}", e)))?;

        for chunk_index in 0..total_chunks {
            let mut chunk_data = vec![0u8; chunk_size];
            let bytes_read = std::io::Read::read(&mut file, &mut chunk_data)
                .map_err(|e| crate::error::MessengerError::File(format!("Failed to read file chunk: {}", e)))?;

            chunk_data.truncate(bytes_read);

            let message = Message {
                id: Uuid::new_v4(),
                message_type: crate::types::MessageType::File {
                    name: file_name.clone(),
                    size: metadata.len(),
                    mime_type: mime_type.clone(),
                    data: Some(chunk_data),
                    chunk_index: Some(chunk_index),
                    total_chunks: Some(total_chunks),
                },
                timestamp: chrono::Utc::now(),
                sender_id: Uuid::new_v4(),
                recipient_id: None,
                status: crate::types::MessageStatus::Sending,
                encrypted: false,
                retry_count: 0,
                metadata: std::collections::HashMap::new(),
            };

            // Store and send chunk
            {
                let mut storage = state.storage.write().await;
                storage.store_message(message.clone()).await?;
            }

            {
                let network_manager = state.network_manager.read().await;
                if let Some(manager) = network_manager.as_ref() {
                    manager.send_message(message).await?;
                } else {
                    return Err(crate::error::MessengerError::NotConnected);
                }
            }

            info!("Sent chunk {}/{} of file {}", chunk_index + 1, total_chunks, file_name);
        }

        info!("File sent in chunks: {}", file_id);
        Ok(file_id)
    }
}

/// Export messages
#[tauri::command]
pub fn export_messages(
    format: ExportFormat,
    _include_metadata: Option<bool>,
    _include_system_messages: Option<bool>,
    state: State<'_, AppState>,
) -> Result<String> {
    info!("Exporting messages in {:?} format", format);

    // For now, return a mock path
    // TODO: Implement actual message export
    Ok("exported_messages.json".to_string())
}

/// Get message statistics
#[tauri::command]
pub fn get_message_stats(_state: State<'_, AppState>) -> Result<crate::storage::StorageStats> {
    // For now, return default stats
    // TODO: Implement actual stats retrieval
    Ok(crate::storage::StorageStats::default())
}

/// Mark message as read
#[tauri::command]
pub fn mark_message_read(
    message_id: Uuid,
    state: State<'_, AppState>,
) -> Result<()> {
    debug!("Marking message as read: {}", message_id);

    // For now, just return success
    // TODO: Implement actual message status update
    Ok(())
}

/// Get unread message count
#[tauri::command]
pub fn get_unread_count(_state: State<'_, AppState>) -> Result<usize> {
    // For now, return 0
    // TODO: Implement actual unread count
    Ok(0)
}
