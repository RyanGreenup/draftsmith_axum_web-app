use crate::server::{handle_not_found, handle_template_error};
use crate::template_context::{NoteTemplateContext, PaginationParams};
use axum::{
    extract::{Path, Query},
    response::{Html, Redirect},
    Form,
};

use crate::flash::{FlashMessage, FlashMessageStore};
use crate::templates::ENV;
use axum::response::{IntoResponse, Response};
use draftsmith_rest_api::client::{update_note, UpdateNoteRequest};
use tower_sessions::Session;

pub async fn route_edit(
    session: Session,
    api_addr: String,
    Path(id): Path<i32>,
    Query(params): Query<PaginationParams>,
) -> Response {
    // Get note data
    let note_handler =
        match NoteTemplateContext::new(session.clone(), Query(params), api_addr, id).await {
            Ok(data) => data,
            Err(e) => {
                eprintln!("Failed to get note data: {:#}", e);
                return handle_not_found(session).await.into_response();
            }
        };

    // Load template
    let template = ENV
        .get_template("body/note/edit.html")
        .unwrap_or_else(|e| panic!("Failed to load template. Error: {:#}", e));

    // Render the template
    let rendered = template
        .render(note_handler.ctx)
        .unwrap_or_else(handle_template_error);

    Html(rendered).into_response()
}

pub async fn route_update_note(
    session: Session,
    api_addr: String,
    Path(path): Path<i32>,
    Form(note): Form<UpdateNoteRequest>,
) -> Redirect {
    let id = path;

    match update_note(&api_addr, id, note).await {
        Ok(_) => {
            session
                .set_flash(FlashMessage::success("Note updated successfully"))
                .await
                .unwrap();
        }
        Err(e) => {
            session
                .set_flash(FlashMessage::error(format!("Failed to update note: {}", e)))
                .await
                .unwrap();
        }
    }

    Redirect::to(&format!("/note/{id}"))
}
