use crate::state::AppState;
use crate::template_context::{BodyTemplateContext, PaginationParams};
use crate::templates::{handle_template_error, ENV};
use axum::{
    extract::{Query, State},
    response::Html,
};
use draftsmith_rest_api::client::notes::{fetch_notes, fts_search_notes, get_note_rendered_html};
use minijinja::context;
use serde::Deserialize;
use tower_sessions::Session;

#[derive(Debug, Deserialize)]
pub struct SearchParams {
    #[serde(flatten)]
    pagination: PaginationParams,
    q: Option<String>, // Search query parameter
}

pub async fn search(
    session: Session,
    State(state): State<AppState>,
    Query(params): Query<SearchParams>,
) -> Html<String> {
    let api_addr: String = state.api_addr.clone();

    // Get the body data
    let body_handler =
        match BodyTemplateContext::new(session, Query(params.pagination), api_addr.clone(), None)
            .await
        {
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

    // Include only the last 50 notes
    let mut recent_notes = notes.into_iter().take(50).collect::<Vec<_>>();

    // // Render the markdown
    // I decided against this because the markdown rendering is slow and
    // any js in there becomes a security risk / chaos with so many
    // pages being rendered at once
    // for note in &mut recent_notes {
    //     // Sanitize the content (nested javascript becomes a nightmare here)
    //     note.content = note.content.replace("<script>", "&lt;script&gt;");
    //     note.content = note.content.replace("</script>", "&lt;/script&gt;");
    //
    //     // Render it
    //     note.content = get_note_rendered_html(&api_addr, note.id)
    //         .await
    //         .unwrap_or(note.content.clone());
    // }

    let template = ENV
        .get_template("body/search_results.html")
        .unwrap_or_else(|e| {
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
