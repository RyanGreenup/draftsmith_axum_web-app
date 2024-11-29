use axum::{
    extract::{Path, Query, State},
    response::{IntoResponse, Response, Redirect},
};
use crate::flash::{FlashMessage, FlashMessageStore};
use crate::state::AppState;
use draftsmith_rest_api::client::{create_note, attach_child_note, AttachChildRequest, fetch_note, CreateNoteRequest, get_note_breadcrumbs};
use tower_sessions::Session;

#[derive(Debug, Default, serde::Deserialize)]
pub struct CreateNoteParams {
    #[serde(default)]
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
            let mut flash_string = format!("Note created successfully #{}", note.id);

            // Handle different creation modes
            match (parent_id, params.as_sibling) {
                // Create as sibling of specified note
                (Some(reference_id), true) => {
                    if let Ok(reference_note) = fetch_note(&api_addr, reference_id, true).await {
                        let mut breadcrumbs = get_note_breadcrumbs(&api_addr, reference_id).await.unwrap();
                        // The last element is the current note
                        breadcrumbs.pop();
                        // TODO handle optional
                        let parent_note = breadcrumbs.pop();
                        if let Some(parent_note) = parent_note {
                            let attach_request = AttachChildRequest {
                                child_note_id: note.id,
                                parent_note_id: Some(parent_note.id),

                            };
                            match attach_child_note(&api_addr, attach_request).await {
                                Ok(_) => {
                                    flash_string.push_str(&format!(" as sibling of #{}", reference_id));
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
                    let attach_request = AttachChildRequest {
                        child_note_id: note.id,
                        parent_note_id: Some(parent_id),

                    };
                    match attach_child_note(&api_addr, attach_request).await {
                        Ok(_) => {
                            flash_string.push_str(&format!(" and attached to #{}", parent_id));
                        }
                        Err(e) => {
                            flash_string.push_str(&format!(" but failed to attach to #{}", e));
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
