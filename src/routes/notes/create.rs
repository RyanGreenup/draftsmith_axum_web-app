use axum::{
    extract::{Path, State},
    response::{IntoResponse, Response, Redirect},
};
use crate::flash::{FlashMessage, FlashMessageStore};
use crate::state::AppState;
use draftsmith_rest_api::client::{create_note, attach_child_note, CreateNoteRequest};
use tower_sessions::Session;

pub async fn route_create(
    session: Session,
    State(state): State<AppState>,
    Path(parent_id): Path<Option<i32>>,
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

            // If there is a parent, attach this note to it
            if let Some(parent_id) = parent_id {
                match attach_note(&api_addr, parent_id, note.id).await {
                    Ok(_) => {
                        flash_string.push_str(&format!(" and attached to note {}", parent_id));
                    }
                    Err(e) => {
                        flash_string.push_str(&format!(" but failed to attach to parent: {}", e));
                    }
                }
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
