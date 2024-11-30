use axum::{
    extract::{Path, State, Query},
    response::{Html, Redirect, Response, IntoResponse},
    Form,
};
use draftsmith_rest_api::client::tags::{list_tags, list_note_tags, attach_tag_to_note, detach_tag_from_note};
use crate::state::AppState;
use crate::flash::{FlashMessage, FlashMessageStore};
use crate::template_context::{NoteTemplateContext, PaginationParams};
use minijinja::context;
use serde::Deserialize;
use tower_sessions::Session;
use crate::templates::{handle_not_found, handle_template_error, ENV};

#[derive(Debug, Deserialize)]
pub struct TagActionForm {
    tag_id: i32,
    action: String, // "attach" or "detach"
}

pub async fn route_assign_tags_get(
    session: Session,
    State(state): State<AppState>,
    Path(note_id): Path<i32>,
    Query(params): Query<PaginationParams>,
) -> Response {
    let api_addr = state.api_addr.clone();

    // Get note data
    let note_handler =
        match NoteTemplateContext::new(session.clone(), Query(params), api_addr.clone(), note_id).await {
            Ok(data) => data,
            Err(e) => {
                eprintln!("Failed to get note data: {:#}", e);
                return handle_not_found(session).await.into_response();
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


    let ctx = context! { ..note_handler.ctx, ..context! {
        note_id => note_id,
        all_tags => all_tags,
        note_tags => note_tags,
    }};


    let rendered = template.render(ctx).unwrap_or_else(handle_template_error);
    Html(rendered).into_response()
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
