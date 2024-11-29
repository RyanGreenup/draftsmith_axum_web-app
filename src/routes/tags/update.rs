use axum::{
    extract::{State, Path},
    response::Redirect,
    Form,
};
use tower_sessions::Session;
use draftsmith_rest_api::client::tags::{update_tag, UpdateTagRequest, attach_child_tag, detach_child_tag};
use crate::flash::{FlashMessage, FlashMessageStore};
use crate::state::AppState;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct SetParentRequest {
    parent_id: Option<i32>,
}

#[axum::debug_handler]
pub async fn route_update_tag(
    session: Session,
    State(state): State<AppState>,
    Path(id): Path<i32>,
    Form(form): Form<UpdateTagRequest>,
) -> Redirect {
    match update_tag(&state.api_addr, id, form).await {
        Ok(_) => {
            session
                .set_flash(FlashMessage::success("Tag updated successfully"))
                .await
                .unwrap();
        }
        Err(e) => {
            session
                .set_flash(FlashMessage::error(format!("Failed to update tag: {}", e)))
                .await
                .unwrap();
        }
    }

    Redirect::to("/manage_tags")
}

#[axum::debug_handler]
pub async fn route_set_parent(
    session: Session,
    State(state): State<AppState>,
    Path(child_id): Path<i32>,
    Form(form): Form<SetParentRequest>,
) -> Redirect {
    // First detach from current parent if any
    if let Err(e) = detach_child_tag(&state.api_addr, child_id).await {
        session
            .set_flash(FlashMessage::error(format!("Failed to detach tag from current parent: {}", e)))
            .await
            .unwrap();
        return Redirect::to("/manage_tags");
    }

    // If a new parent is specified, attach to it
    if let Some(parent_id) = form.parent_id {
        match attach_child_tag(&state.api_addr, parent_id, child_id).await {
            Ok(_) => {
                session
                    .set_flash(FlashMessage::success("Tag hierarchy updated successfully"))
                    .await
                    .unwrap();
            }
            Err(e) => {
                session
                    .set_flash(FlashMessage::error(format!("Failed to attach tag to new parent: {}", e)))
                    .await
                    .unwrap();
            }
        }
    } else {
        session
            .set_flash(FlashMessage::success("Tag detached successfully"))
            .await
            .unwrap();
    }

    Redirect::to("/manage_tags")
}
