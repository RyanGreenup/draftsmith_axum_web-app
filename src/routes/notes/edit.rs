use crate::server::{handle_not_found, handle_template_error};
use crate::template_context::{NoteTemplateContext, PaginationParams};
use crate::templates::ENV;
use axum::{
    extract::{Path, Query},
    response::{Html, IntoResponse, Response},
};
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
