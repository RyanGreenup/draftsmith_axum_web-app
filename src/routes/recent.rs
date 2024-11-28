use crate::state::AppState;
use crate::template_context::{BodyTemplateContext, PaginationParams};
use crate::templates::{handle_template_error, ENV};
use axum::{
    extract::{Query, State},
    response::Html,
};
use draftsmith_rest_api::client::notes::fetch_notes;
use minijinja::context;
use tower_sessions::Session;

// TODO implement recent
pub async fn route_recent(
    session: Session,
    State(state): State<AppState>,
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

    // Get Recent notes
    let metadata_only = true;
    let mut notes = match fetch_notes(&api_addr, metadata_only).await {
        Ok(notes) => notes,
        Err(e) => {
            eprintln!("Failed to fetch notes: {:#?}", e);
            return Html(String::from("<h1>Error fetching notes</h1>"));
        }
    };

    // Sort notes by updated_at
    notes.sort_by(|a, b| a.modified_at.cmp(&b.modified_at));

    // Include only the last 50 notes
    let recent_notes = notes.iter().rev().take(50).collect::<Vec<_>>();

    let template = ENV.get_template("body/recent.html").unwrap_or_else(|e| {
        panic!("Failed to load template. Error: {:#}", e);
    });

    // get the context vars
    let ctx = context! { ..body_handler.ctx, ..context! {
        recent_notes => recent_notes,
    }};

    let rendered = template.render(ctx).unwrap_or_else(handle_template_error);

    Html(rendered)
}
