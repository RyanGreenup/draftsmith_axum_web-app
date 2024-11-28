use crate::state::AppState;
use crate::template_context::{NoteTemplateContext, PaginationParams};
use crate::templates::{handle_not_found, handle_template_error, ENV};
use axum::{
    extract::{Path, Query, State},
    response::{Html, IntoResponse, Response},
};
use minijinja::context;
use tower_sessions::Session;

pub async fn route_note(
    session: Session,
    State(state): State<AppState>,
    Path(id): Path<i32>,
    Query(params): Query<PaginationParams>,
) -> Response {
    let api_addr: String = state.api_addr.clone();
    // Get note data
    let note_handler =
        match NoteTemplateContext::new(session.clone(), Query(params), api_addr, id).await {
            Ok(data) => data,
            Err(e) => {
                eprintln!("Failed to get note data: {:#}", e);
                return handle_not_found(session).await.into_response();
            }
        };

    // Get rendered HTML
    let rendered_note = match note_handler.get_rendered_html(id).await {
        Ok(html) => html,
        Err(e) => {
            eprintln!("Failed to get rendered note: {:#}", e);
            return Html(String::from("<h1>Error rendering note</h1>")).into_response();
        }
    };

    // Load and render template
    let template = ENV.get_template("body/note/read.html").unwrap_or_else(|e| {
        panic!("Failed to load template. Error: {:#}", e);
    });

    let ctx = context! { ..note_handler.ctx, ..context! {
        rendered_note => rendered_note,
    }};

    let rendered = match template.render(ctx) {
        Ok(result) => result,
        Err(err) => handle_template_error(err),
    };

    Html(rendered).into_response()
}
