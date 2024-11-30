use axum::{
    extract::{State, Path},
    response::Redirect,
    Form,
};
use tower_sessions::Session;
use draftsmith_rest_api::client::tags::{update_tag, UpdateTagRequest, attach_child_tag, detach_child_tag, get_tag_tree, TagTreeNode};
use crate::flash::{FlashMessage, FlashMessageStore};
use crate::state::AppState;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct SetParentRequest {
    parent_id: Option<String>, // Change to String to handle empty string from form
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
    // Get the full tag tree to check if tag has a parent
    let tag_tree = match get_tag_tree(&state.api_addr).await {
        Ok(tree) => tree,
        Err(e) => {
            session
                .set_flash(FlashMessage::error(format!("Failed to get tag tree: {}", e)))
                .await
                .unwrap();
            return Redirect::to("/manage_tags");
        }
    };

    // Helper function to check if a tag has a parent in the tree
    fn has_parent_in_tree(tree: &[TagTreeNode], child_id: i32) -> bool {
        for node in tree {
            // Check if this tag is a parent of our target
            if node.children.iter().any(|child| child.id == child_id) {
                return true;
            }
            // Recursively check children
            if has_parent_in_tree(&node.children, child_id) {
                return true;
            }
        }
        false
    }

    // Only try to detach if the tag actually has a parent
    if has_parent_in_tree(&tag_tree, child_id) {
        if let Err(e) = detach_child_tag(&state.api_addr, child_id).await {
            session
                .set_flash(FlashMessage::error(format!("Failed to detach tag from current parent: {}", e)))
                .await
                .unwrap();
            return Redirect::to("/manage_tags");
        }
    }

    // Convert empty string to None, otherwise parse the string to i32
    let parent_id = form.parent_id
        .filter(|s| !s.is_empty())
        .map(|s| s.parse::<i32>().unwrap_or(0));

    // If a new parent is specified, attach to it
    if let Some(parent_id) = parent_id {
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
            .set_flash(FlashMessage::success("Tag hierarchy updated successfully"))
            .await
            .unwrap();
    }

    Redirect::to("/manage_tags")
}
