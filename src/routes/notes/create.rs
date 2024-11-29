use axum::{
    extract::{Path, Query, State},
    response::{IntoResponse, Response, Redirect},
};
use crate::flash::{FlashMessage, FlashMessageStore};
use crate::state::AppState;
use draftsmith_rest_api::client::{
    create_note, attach_child_note, AttachChildRequest,
    fetch_note, CreateNoteRequest, get_note_breadcrumbs,
};
use tower_sessions::Session;

#[derive(Debug, Default, serde::Deserialize)]
pub struct CreateNoteParams {
    #[serde(default)]
    as_sibling: bool,
}

async fn handle_attachment(
    api_addr: &str,
    note_id: i32,
    reference_id: i32,
    as_sibling: bool,
) -> Result<String, String> {
    if as_sibling {
        let breadcrumbs = get_note_breadcrumbs(api_addr, reference_id)
            .await
            .map_err(|e| format!("Failed to get breadcrumbs: {}", e))?;

        // Get parent from breadcrumbs (excluding current note and its parent)
        let parent_id = breadcrumbs
            .iter()
            .rev()
            .nth(1)
            .map(|note| note.id)
            .ok_or_else(|| "No parent found for sibling".to_string())?;

        attach_note(api_addr, note_id, parent_id).await?;
        Ok(format!(" as sibling of #{}", reference_id))
    } else {
        attach_note(api_addr, note_id, reference_id).await?;
        Ok(format!(" and attached to #{}", reference_id))
    }
}

async fn attach_note(api_addr: &str, child_id: i32, parent_id: i32) -> Result<(), String> {
    let attach_request = AttachChildRequest {
        child_note_id: child_id,
        parent_note_id: Some(parent_id),
    };

    attach_child_note(api_addr, attach_request)
        .await
        .map_err(|e| format!("Failed to attach note: {}", e))
}

pub async fn route_create(
    session: Session,
    State(state): State<AppState>,
    Path(parent_id): Path<Option<i32>>,
    Query(params): Query<CreateNoteParams>,
) -> Response {
    let create_request = CreateNoteRequest {
        title: String::new(),
        content: String::new(),
    };

    match create_note(&state.api_addr, create_request).await {
        Ok(note) => {
            let mut message = format!("Note created successfully #{}", note.id);

            // Handle attachment if parent_id is provided
            if let Some(reference_id) = parent_id {
                if let Err(e) = handle_attachment(&state.api_addr, note.id, reference_id, params.as_sibling).await {
                    message.push_str(&format!(" but failed to attach: {}", e));
                }
            }

            session
                .set_flash(FlashMessage::success(message))
                .await
                .unwrap_or_else(|e| eprintln!("Failed to set flash message: {}", e));

            Redirect::to(&format!("/edit/{}", note.id)).into_response()
        }
        Err(e) => {
            session
                .set_flash(FlashMessage::error(format!("Failed to create note: {}", e)))
                .await
                .unwrap_or_else(|e| eprintln!("Failed to set flash message: {}", e));

            Redirect::to("/").into_response()
        }
    }
}
