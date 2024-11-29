use axum::{
    extract::State,
    response::Redirect,
    Form,
};

use tower_sessions::Session;
use draftsmith_rest_api::client::tags::{create_tag, CreateTagRequest};
use crate::flash::{FlashMessage, FlashMessageStore};
use crate::state::AppState;

pub async fn route_create_tag(
    session: Session,
    State(state): State<AppState>,
    Form(form): Form<CreateTagRequest>,
) -> Redirect {
    match create_tag(&state.api_addr, form).await {
        Ok(_) => {
            session
                .set_flash(FlashMessage::success("Tag created successfully"))
                .await
                .unwrap();
        }
        Err(e) => {
            session
                .set_flash(FlashMessage::error(format!("Failed to create tag: {}", e)))
                .await
                .unwrap();
        }
    }

    Redirect::to("/manage_tags")
}
