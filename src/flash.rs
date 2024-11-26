use axum::async_trait;
use serde::{Deserialize, Serialize};
use tower_sessions::{Session, session::Error as SessionError};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FlashMessage {
    pub kind: String,    // "success", "error", "info", "warning"
    pub message: String,
}

impl FlashMessage {
    pub fn success(message: impl Into<String>) -> Self {
        Self {
            kind: "success".to_string(),
            message: message.into(),
        }
    }

    pub fn error(message: impl Into<String>) -> Self {
        Self {
            kind: "error".to_string(),
            message: message.into(),
        }
    }

    pub fn info(message: impl Into<String>) -> Self {
        Self {
            kind: "info".to_string(),
            message: message.into(),
        }
    }

    pub fn warning(message: impl Into<String>) -> Self {
        Self {
            kind: "warning".to_string(),
            message: message.into(),
        }
    }
}

#[async_trait]
pub trait FlashMessageStore {
    async fn set_flash(&self, message: FlashMessage) -> Result<(), SessionError>;
    async fn take_flash(&self) -> Result<Option<FlashMessage>, SessionError>;
}

#[async_trait]
impl FlashMessageStore for Session {
    async fn set_flash(&self, message: FlashMessage) -> Result<(), SessionError> {
        self.insert("flash", message).await
    }

    async fn take_flash(&self) -> Result<Option<FlashMessage>, SessionError> {
        self.remove::<FlashMessage>("flash").await
    }
}
