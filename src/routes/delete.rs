use crate::template_context::{NoteTemplateContext, PaginationParams};
use axum::{
    extract::{Path, Query},
    response::{Html, Redirect},
    Form,
};

use crate::flash::{FlashMessage, FlashMessageStore};
use crate::state::AppState;
use crate::templates::{handle_not_found, handle_template_error, ENV};
use axum::{
    extract::State,
    response::{IntoResponse, Response},
};
use draftsmith_rest_api::client::{UpdateNoteRequest, delete_note};
use tower_sessions::Session;

pub async fd route_delete(
    // TODO
