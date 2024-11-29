use crate::state::AppState;
use crate::template_context::{BodyTemplateContext, PaginationParams};
use crate::templates::{handle_template_error, ENV};
use axum::{
    extract::{Path, Query, State},
    response::Html,
};
use draftsmith_rest_api::client::notes::fetch_notes;
use draftsmith_rest_api::client::tags::{list_note_tags, list_tags};
use minijinja::context;
use tower_sessions::Session;

// TODO implement recent
pub async fn route_list_tag(
    session: Session,
    State(state): State<AppState>,
    Path(id): Path<i32>,
    Query(params): Query<PaginationParams>,
) -> Html<String> {
    let api_addr: String = state.api_addr.clone();
    // Get the body data
    let body_handler =
        match BodyTemplateContext::new(session, Query(params), api_addr.clone(), None).await {
            Ok(handler) => handler,
            Err(e) => {
                eprintln!("Failed to create body handler: {:#?}", e);
                return Html(String::from("<h1>Error getting page data</h1>"));
            }
        };

    // Get the tag name
    let tag = list_tags(&api_addr)
        .await
        .unwrap_or_else(|e| {
            eprintln!("Failed to get tags: {:#?}", e);
            vec![]
        })
        // Filter out the current tag
        .into_iter()
        .filter(|tag| tag.id == id)
        .collect::<Vec<_>>();

    let tag_name = match tag.first() {
        Some(tag) => tag.name.clone(),
        None => String::from("Unkn  "),
    };

    // Get all tag Note Pairings
    let tag_notes: Vec<_> = list_note_tags(&api_addr)
        .await
        .unwrap_or_else(|e| {
            eprintln!("Failed to get tags: {:#?}", e);
            vec![]
        })
        // Filter out the relevant tags
        .into_iter()
        .filter(|tag| tag.tag_id == id)
        .collect();

    // Get all the note Metadata
    let mut notes: Vec<_> = fetch_notes(&api_addr, true)
        .await
        .expect("TODO don't panic")
        .into_iter()
        // Filter out the relevant notes for the tag id
        .filter(|note| tag_notes.iter().any(|tag| tag.note_id == note.id))
        .collect::<Vec<_>>();

    // Sort notes by updated_at
    notes.sort_by(|a, b| a.modified_at.cmp(&b.modified_at));

    let template = ENV.get_template("body/tags/list.html").unwrap_or_else(|e| {
        panic!("Failed to load template. Error: {:#}", e);
    });

    // get the context vars
    let ctx = context! { ..body_handler.ctx, ..context! {
        notes => notes,
        tag_name => tag_name,
    }};

    let rendered = template.render(ctx).unwrap_or_else(handle_template_error);

    Html(rendered)
}
