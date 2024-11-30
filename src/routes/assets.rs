use axum::{
    extract::{Query, State},
    response::Html,
};
use tower_sessions::Session;
use crate::state::AppState;
use crate::template_context::{BodyTemplateContext, PaginationParams};
use crate::templates::{ENV, handle_template_error};
use draftsmith_rest_api::client::assets::list_assets;
use minijinja::context;

pub async fn route_list_assets(
    session: Session,
    State(state): State<AppState>,
    Query(params): Query<PaginationParams>,
) -> Html<String> {
    let api_addr: String = state.api_addr.clone();

    // Get the body data
    let body_handler = match BodyTemplateContext::new(session, Query(params), api_addr.clone(), None).await {
        Ok(handler) => handler,
        Err(e) => {
            eprintln!("Failed to create body handler: {:#?}", e);
            return Html(String::from("<h1>Error getting page data</h1>"));
        }
    };

    // Get assets list
    let assets = match list_assets(&api_addr, None).await {
        Ok(assets) => assets,
        Err(e) => {
            eprintln!("Failed to fetch assets: {:#?}", e);
            return Html(String::from("<h1>Error fetching assets</h1>"));
        }
    };

    let template = ENV.get_template("body/assets.html").unwrap_or_else(|e| {
        panic!("Failed to load template. Error: {:#}", e);
    });

    let ctx = context! {
        ..body_handler.ctx,
        assets => assets,
    };

    let rendered = template.render(ctx).unwrap_or_else(handle_template_error);

    Html(rendered)
}
