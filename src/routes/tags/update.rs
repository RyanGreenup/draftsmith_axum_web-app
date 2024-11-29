use axum::{
    extract::{State, Path},
    response::Redirect,
    Form,
};
use tower_sessions::Session;
use draftsmith_rest_api::client::tags::{update_tag, UpdateTagRequest, attach_child_tag, detach_child_tag};
use crate::flash::{FlashMessage, FlashMessageStore};
use crate::state::AppState;

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
