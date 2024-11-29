use crate::state::AppState;
use crate::template_context::{BodyTemplateContext, PaginationParams};
use crate::templates::{handle_template_error, ENV};
use axum::{
    extract::{Query, State},
    response::Html,
};
use draftsmith_rest_api::client::notes::{fetch_notes, fts_search_notes};
use minijinja::context;
use tower_sessions::Session;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct SearchParams {
    #[serde(flatten)]
    pagination: PaginationParams,
    q: Option<String>,  // Search query parameter
}

pub async fn search(
    session: Session,
    State(state): State<AppState>,
    Query(params): Query<SearchParams>,
) -> Html<String> {
    let api_addr: String = state.api_addr.clone();
    
    // Get the body data
    let body_handler = match BodyTemplateContext::new(
        session, 
        Query(params.pagination), 
        api_addr.clone(), 
        None
    ).await {
        Ok(handler) => handler,
        Err(e) => {
            eprintln!("Failed to create body handler: {:#?}", e);
            return Html(String::from("<h1>Error getting page data</h1>"));
        }
    };

    // Get notes based on whether we have a search term
    let notes = if let Some(ref search_term) = params.q {
        match fts_search_notes(&api_addr, search_term).await {
            Ok(notes) => notes,
            Err(e) => {
                eprintln!("Failed to search notes: {:#?}", e);
                return Html(String::from("<h1>Error searching notes</h1>"));
            }
        }
    } else {
        // If no search term, get recent notes
        let metadata_only = true;
        match fetch_notes(&api_addr, metadata_only).await {
            Ok(notes) => notes,
            Err(e) => {
                eprintln!("Failed to fetch notes: {:#?}", e);
                return Html(String::from("<h1>Error fetching notes</h1>"));
            }
        }
    };

    // Sort notes by updated_at
    let mut sorted_notes = notes;
    sorted_notes.sort_by(|a, b| b.modified_at.cmp(&a.modified_at));

    // Include only the last 50 notes
    let recent_notes = sorted_notes.iter().take(50).collect::<Vec<_>>();

    let template = ENV.get_template("body/recent.html").unwrap_or_else(|e| {
        panic!("Failed to load template. Error: {:#}", e);
    });

    // get the context vars
    let ctx = context! { 
        ..body_handler.ctx, 
        ..context! {
            recent_notes => recent_notes,
            search_term => params.q.unwrap_or_default(),
        }
    };

    let rendered = template.render(ctx).unwrap_or_else(handle_template_error);

    Html(rendered)
}
