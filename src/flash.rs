
use axum::{
    extract::{Path, Query},
    response::{Html, Redirect},
    routing::get,
    Form, Router,
};
use draftsmith_rest_api::client::{
    fetch_note, notes::get_note_rendered_html, update_note, UpdateNoteRequest
};
use include_dir::{include_dir, Dir};
use minijinja::{context, Environment, Error};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use tower_sessions::{MemoryStore, Session, SessionManagerLayer};

#[derive(Debug, Serialize, Deserialize, Clone)]
struct FlashMessage {
    kind: String, // "success", "error", "info", "warning"
    message: String,
}

use axum::async_trait;
use serde::{Deserialize, Serialize};
use tower_sessions::Session;

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
    async fn set_flash(&self, message: FlashMessage) -> Result<(), tower_sessions::Error>;
    async fn take_flash(&self) -> Result<Option<FlashMessage>, tower_sessions::Error>;
}

#[async_trait]
impl FlashMessageStore for Session {
    async fn set_flash(&self, message: FlashMessage) -> Result<(), tower_sessions::Error> {
        self.insert("flash", message).await
    }

    async fn take_flash(&self) -> Result<Option<FlashMessage>, tower_sessions::Error> {
        self.remove::<FlashMessage>("flash").await
    }
}
