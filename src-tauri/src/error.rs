use thiserror::Error;
use tauri::ipc::InvokeError;

/// Main error type for the TCP Messenger application
#[derive(Error, Debug)]
pub enum MessengerError {
    #[error("Network error: {0}")]
    Network(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Encryption error: {0}")]
    Encryption(String),

    #[error("Authentication error: {0}")]
    Authentication(String),

    #[error("Protocol error: {0}")]
    Protocol(String),

    #[error("File error: {0}")]
    File(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Storage error: {0}")]
    Storage(String),

    #[error("Network manager error: {0}")]
    NetworkManager(String),

    #[error("Invalid input: {0}")]
    InvalidInput(String),

    #[error("Connection timeout")]
    ConnectionTimeout,

    #[error("Connection refused")]
    ConnectionRefused,

    #[error("Already connected")]
    AlreadyConnected,

    #[error("Not connected")]
    NotConnected,

    #[error("Server not running")]
    ServerNotRunning,

    #[error("Client not connected")]
    ClientNotConnected,

    #[error("Message too large: {size} bytes (max: {max})")]
    MessageTooLarge { size: usize, max: usize },

    #[error("Invalid message type: {0}")]
    InvalidMessageType(String),

    #[error("Key exchange failed: {0}")]
    KeyExchangeFailed(String),

    #[error("Decryption failed: {0}")]
    DecryptionFailed(String),

    #[error("File transfer error: {0}")]
    FileTransferError(String),

    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    #[error("Resource not found: {0}")]
    ResourceNotFound(String),

    #[error("Operation not supported: {0}")]
    OperationNotSupported(String),

    #[error("Internal error: {0}")]
    Internal(String),
}

/// Result type alias for the application
pub type Result<T> = std::result::Result<T, MessengerError>;

impl From<MessengerError> for String {
    fn from(err: MessengerError) -> Self {
        err.to_string()
    }
}

impl From<MessengerError> for InvokeError {
    fn from(err: MessengerError) -> Self {
        InvokeError::from(err.to_string())
    }
}

/// Helper trait for converting errors to MessengerError
pub trait IntoMessengerError<T> {
    fn into_messenger_error(self, context: &str) -> Result<T>;
}

impl<T, E> IntoMessengerError<T> for std::result::Result<T, E>
where
    E: std::fmt::Display,
{
    fn into_messenger_error(self, context: &str) -> Result<T> {
        self.map_err(|e| MessengerError::Internal(format!("{}: {}", context, e)))
    }
}

/// Macro for creating internal errors with context
#[macro_export]
macro_rules! internal_error {
    ($msg:expr) => {
        MessengerError::Internal($msg.to_string())
    };
    ($fmt:expr, $($arg:tt)*) => {
        MessengerError::Internal(format!($fmt, $($arg)*))
    };
}

/// Macro for creating protocol errors
#[macro_export]
macro_rules! protocol_error {
    ($msg:expr) => {
        MessengerError::Protocol($msg.to_string())
    };
    ($fmt:expr, $($arg:tt)*) => {
        MessengerError::Protocol(format!($fmt, $($arg)*))
    };
}

/// Macro for creating encryption errors
#[macro_export]
macro_rules! encryption_error {
    ($msg:expr) => {
        MessengerError::Encryption($msg.to_string())
    };
    ($fmt:expr, $($arg:tt)*) => {
        MessengerError::Encryption(format!($fmt, $($arg)*))
    };
}
