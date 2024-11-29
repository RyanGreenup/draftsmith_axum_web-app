use axum::{
    extract::{Path, Query, State},
    response::{IntoResponse, Response, Redirect},
};
use crate::flash::{FlashMessage, FlashMessageStore};
use crate::state::AppState;
use draftsmith_rest_api::client::{create_note, attach_child_note, get_note, CreateNoteRequest};
use tower_sessions::Session;

#[derive(Debug, Default, serde::Deserialize)]
pub struct CreateNoteParams {
    as_sibling: bool,
}

pub async fn route_create(
    session: Session,
    State(state): State<AppState>,
    Path(parent_id): Path<Option<i32>>,
    Query(params): Query<CreateNoteParams>,
) -> Response {
    let api_addr: String = state.api_addr.clone();

    let create_request = CreateNoteRequest {
        // Title has been dropped from the api in favour of H1
        title: "".to_string(),
        content: "".to_string(),
    };

    match create_note(&api_addr, create_request).await {
        Ok(note) => {
            // Build flash message
            let mut flash_string = "Note created successfully".to_string();

            // Handle different creation modes
            match (parent_id, params.as_sibling) {
                // Create as sibling of specified note
                (Some(reference_id), true) => {
                    if let Ok(reference_note) = get_note(&api_addr, reference_id).await {
                        if let Some(parent_id) = reference_note.parent_id {
                            match attach_note(&api_addr, parent_id, note.id).await {
                                Ok(_) => {
                                    flash_string.push_str(&format!(" as sibling of note {}", reference_id));
                                }
                                Err(e) => {
                                    flash_string.push_str(&format!(" but failed to attach as sibling: {}", e));
                                }
                            }
                        }
                    }
                },
                // Create as child of specified note
                (Some(parent_id), false) => {
                    match attach_note(&api_addr, parent_id, note.id).await {
                        Ok(_) => {
                            flash_string.push_str(&format!(" and attached to note {}", parent_id));
                        }
                        Err(e) => {
                            flash_string.push_str(&format!(" but failed to attach to parent: {}", e));
                        }
                    }
                },
                // Create standalone note
                (None, _) => {}
            }

            // Set the flash
            session
                .set_flash(FlashMessage::success(flash_string))
                .await
                .unwrap();
            // Redirect
            Redirect::to(&format!("/edit/{}", note.id)).into_response()
        }
        Err(e) => {
            session
                .set_flash(FlashMessage::error(format!("Failed to create note: {}", e)))
                .await
                .unwrap();
            Redirect::to("/").into_response()
        }
    }
}
