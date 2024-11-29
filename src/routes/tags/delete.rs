use axum::{
    extract::{State, Path},
    response::Redirect,
};
use tower_sessions::Session;
use draftsmith_rest_api::client::tags::delete_tag;
use crate::flash::{FlashMessage, FlashMessageStore};
use crate::state::AppState;

pub async fn route_delete_tag(
    session: Session,
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Redirect {
    match delete_tag(&state.api_addr, id).await {
        Ok(_) => {
            session
                .set_flash(FlashMessage::success("Tag deleted successfully"))
                .await
                .unwrap();
        }
        Err(e) => {
            session
                .set_flash(FlashMessage::error(format!("Failed to delete tag: {}", e)))
                .await
                .unwrap();
        }
    }

    Redirect::to("/manage_tags")
}
