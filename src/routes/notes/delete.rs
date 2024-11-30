use axum::{
    extract::{Path, State},
    response::{IntoResponse, Response, Redirect},
};
use crate::flash::{FlashMessage, FlashMessageStore};
use crate::state::AppState;
use draftsmith_rest_api::client::delete_note;
use tower_sessions::Session;

pub async fn route_delete(
    session: Session,
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Response {
    let api_addr: String = state.api_addr.clone();

    // Add confirmation via query param
    if !session.get::<bool>("confirm_delete").await.unwrap_or(Some(false)).unwrap_or(false) {
        session.insert("confirm_delete", true).await.unwrap();
        return Redirect::to(&format!("/note/{id}")).into_response();
    }
    
    session.remove::<bool>("confirm_delete").await.unwrap();

    match delete_note(&api_addr, id).await {
        Ok(_) => {
            session
                .set_flash(FlashMessage::success("Note deleted successfully"))
                .await
                .unwrap();
            
            Redirect::to("/").into_response()
        }
        Err(e) => {
            session
                .set_flash(FlashMessage::error(format!("Failed to delete note: {}", e)))
                .await
                .unwrap();
            
            Redirect::to(&format!("/note/{id}")).into_response()
        }
    }
}
