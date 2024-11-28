use crate::html_builder::build_note_tree_html;
use crate::templates::{handle_template_error, ENV};
use serde::Deserialize;
use crate::template_context::{NoteTemplateContext, PaginationParams};

use crate::flash::{FlashMessage, FlashMessageStore};
use crate::state::AppState;
use crate::MAX_ITEMS_PER_PAGE;
use draftsmith_rest_api::client::{
    attach_child_note, detach_child_note, fetch_note_tree, get_note_breadcrumbs,
    AttachChildRequest, NoteBreadcrumb,
};
use minijinja::context;
use tower_sessions::Session;
use crate::templates::handle_not_found;
use axum::{
    extract::{Path, Query, State},
    response::{Html, IntoResponse, Response, Redirect},
    Form,
};

pub async fn route_move_note_get(
    session: Session,
    State(state): State<AppState>,
    Path(note_id): Path<i32>,
    Query(params): Query<PaginationParams>,
) -> Response {
    let api_addr: String = state.api_addr.clone();

    // Get note data
    let note_handler =
        match NoteTemplateContext::new(session.clone(), Query(params), api_addr, note_id).await {
            Ok(data) => data,
            Err(e) => {
                eprintln!("Failed to get note data: {:#}", e);
                return handle_not_found(session).await.into_response();
            }
        };


    // Load and render template
    let template = ENV.get_template("body/note/move.html").unwrap_or_else(|e| {
        panic!("Failed to load template. Error: {:#}", e);
    });

    let rendered = template
        .render(note_handler.ctx)
        .unwrap_or_else(handle_template_error);

    Html(rendered).into_response()
}

pub async fn route_detach_note_post(
    session: Session,
    State(state): State<AppState>,
    Path(note_id): Path<i32>,
) -> Redirect {
    let api_addr: String = state.api_addr.clone();

    match detach_child_note(&api_addr, note_id).await {
        Ok(_) => {
            session
                .set_flash(FlashMessage::success("Note detached successfully"))
                .await
                .unwrap();
        }
        Err(e) => {
            session
                .set_flash(FlashMessage::error(format!("Failed to detach note: {}", e)))
                .await
                .unwrap();
        }
    }

    Redirect::to(&format!("/note/{note_id}"))
}

pub async fn route_move_note_post(
    session: Session,
    State(state): State<AppState>,
    Path(note_id): Path<i32>,
    Form(form): Form<MoveNoteForm>,
) -> Redirect {
    let api_addr: String = state.api_addr.clone();

    // Get the breadcrumbs to check for parents
    let breadcrumbs: Option<Vec<NoteBreadcrumb>> =
        match get_note_breadcrumbs(&api_addr, note_id).await {
            Ok(b) => Some(b),
            Err(e) => {
                eprintln!("Failed to get Note Breadcrumbs: {:#?}", e);
                None
            }
        };

    // Detach the note from its current parent if it has one
    if let Some(bc) = breadcrumbs {
        if bc.len() > 1 {
            match detach_child_note(&api_addr, note_id).await {
                Ok(_) => (),
                Err(e) => {
                    session
                        .set_flash(FlashMessage::error(format!("Failed to detach note: {}", e)))
                        .await
                        .unwrap_or_else(|e| {
                            eprintln!("Failed to set flash message: {:#?}", e);
                        });
                    return Redirect::to(&format!("/note/{note_id}"));
                }
            }
        }
    }

    // NOTE attaching to id=0 will effectively detach
    // I am relying on this in ../static/js/controllers/tree_controller.js
    // Probably a candidate for a refactor
    // Then attach it to the new parent
    let attach_request = AttachChildRequest {
        parent_note_id: Some(form.new_parent_id),
        child_note_id: note_id,
    };

    // Flash the result
    match attach_child_note(&api_addr, attach_request).await {
        Ok(_) => {
            session
                .set_flash(FlashMessage::success("Note moved successfully"))
                .await
                .unwrap();
        }
        Err(e) => {
            session
                .set_flash(FlashMessage::error(format!("Failed to move note: {}", e)))
                .await
                .unwrap();
        }
    }

    Redirect::to(&format!("/note/{note_id}"))
}

#[derive(Deserialize)]
pub struct MoveNoteForm {
    pub new_parent_id: i32,
}
