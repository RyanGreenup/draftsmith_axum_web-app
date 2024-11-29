use crate::state::AppState;
use crate::template_context::{BodyTemplateContext, PaginationParams};
use crate::templates::{handle_template_error, ENV};
use axum::{
    extract::{Query, State},
    response::Html,
};
use draftsmith_rest_api::client::notes::{fetch_notes, get_note_path};
use draftsmith_rest_api::client::tags::{list_tags, create_tag, delete_tag, update_tag};

use minijinja::context;
use tower_sessions::Session;

// TODO implement recent
pub async fn route_manage_tags(
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

    // Get the tags
    let tags = list_tags(&api_addr).await.unwrap_or_else(|e| {
        eprintln!("Failed to get tags: {:#?}", e);
        vec![]
    });


    let template = ENV.get_template("body/tags/manage_all.html").unwrap_or_else(|e| {
        panic!("Failed to load template. Error: {:#}", e);
    });

    // get the context vars
    let ctx = context! { ..body_handler.ctx, ..context! {
        tags => tags,
    }};

    let rendered = template.render(ctx).unwrap_or_else(handle_template_error);

    Html(rendered)
}
