use axum::{
    extract::{Path, State, Query},
    response::{Html, Redirect},
    Form,
};
use draftsmith_rest_api::client::tags::{list_tags, list_note_tags, attach_tag_to_note, detach_tag_from_note};
use crate::state::AppState;
use crate::flash::{FlashMessage, FlashMessageStore};
use crate::template_context::{BodyTemplateContext, PaginationParams}; 
use crate::templates::{handle_template_error, ENV};
use minijinja::context;
use serde::Deserialize;
use tower_sessions::Session;

#[derive(Debug, Deserialize)]
pub struct TagActionForm {
    tag_id: i32,
    action: String, // "attach" or "detach"
}

pub async fn route_assign_tags_get(
    session: Session,
    State(state): State<AppState>,
    Path(note_id): Path<i32>,
) -> Html<String> {
    let api_addr = state.api_addr.clone();
    
    // Create empty pagination params
    let pagination = Query(PaginationParams::default());
    
    // Get the body context
    let body_handler = match BodyTemplateContext::new(session, pagination, api_addr.clone(), None).await {
        Ok(handler) => handler,
        Err(e) => {
            eprintln!("Failed to create body handler: {:#?}", e);
            return Html(String::from("<h1>Error getting page data</h1>"));
        }
    };

    // Get all available tags
    let all_tags = list_tags(&api_addr)
        .await
        .unwrap_or_else(|e| {
            eprintln!("Failed to get tags: {:#?}", e);
            vec![]
        });

    // Get current note's tags
    let note_tags = list_note_tags(&api_addr)
        .await
        .unwrap_or_else(|e| {
            eprintln!("Failed to get note tags: {:#?}", e);
            vec![]
        })
        .into_iter()
        .filter(|nt| nt.note_id == note_id)
        .collect::<Vec<_>>();

    let template = ENV.get_template("body/note/assign_tags.html").unwrap_or_else(|e| {
        panic!("Failed to load template. Error: {:#}", e);
    });

    let ctx = context! {
        body_handler.ctx,
        note_id: note_id,
        all_tags: all_tags,
        note_tags: note_tags,
    };

    let rendered = template.render(ctx).unwrap_or_else(handle_template_error);
    Html(rendered)
}

pub async fn route_assign_tags_post(
    session: Session,
    State(state): State<AppState>,
    Path(note_id): Path<i32>,
    Form(form): Form<TagActionForm>,
) -> Redirect {
    let api_addr = state.api_addr.clone();

    let result: Result<(), _> = match form.action.as_str() {
        "attach" => {
            attach_tag_to_note(&api_addr, note_id, form.tag_id).await.map(|_| ())
        }
        "detach" => detach_tag_from_note(&api_addr, note_id, form.tag_id).await,
        _ => {
            session
                .set_flash(FlashMessage::error("Invalid action specified"))
                .await
                .unwrap();
            return Redirect::to(&format!("/assign_tags/{}", note_id));
        }
    };

    match result {
        Ok(_) => {
            session
                .set_flash(FlashMessage::success("Tags updated successfully"))
                .await
                .unwrap();
        }
        Err(e) => {
            session
                .set_flash(FlashMessage::error(format!("Failed to update tags: {}", e)))
                .await
                .unwrap();
        }
    }

    Redirect::to(&format!("/assign_tags/{}", note_id))
}
