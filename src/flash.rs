
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

