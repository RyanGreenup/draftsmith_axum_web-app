use crate::state::AppState;
use crate::template_context::{BodyTemplateContext, PaginationParams};
use crate::templates::{handle_template_error, ENV};
use axum::{
    extract::{Path, Query, State},
    response::{Html, Response, IntoResponse},
    http::StatusCode,
};
use draftsmith_rest_api::client::assets::{list_assets, delete_asset};
use minijinja::context;
use tower_sessions::Session;
use crate::flash::{FlashMessage, FlashMessageStore};

pub async fn route_list_assets(
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

    let ctx = context! { ..body_handler.ctx, ..context! {
        assets => assets,
    }};

    let rendered = template.render(ctx).unwrap_or_else(handle_template_error);

    Html(rendered)
}

pub async fn route_edit_asset(
    session: Session,
    State(state): State<AppState>,
    Path(asset_id): Path<i32>,
) -> impl IntoResponse {
    let _ = session.set_flash(FlashMessage::info(
        "Editing assets is not currently supported. Please delete this asset and upload a new one if you need to make changes."
    )).await;

    // Redirect back to assets list
    Response::builder()
        .status(StatusCode::FOUND)
        .header("Location", "/assets")
        .body(axum::body::Body::empty())
        .unwrap()
}

pub async fn route_delete_asset(
    State(state): State<AppState>,
    Path(asset_id): Path<i32>,
    session: Session,
) -> impl IntoResponse {
    match delete_asset(&state.api_addr, asset_id).await {
        Ok(()) => {
            let _ = session.set_flash(FlashMessage::success("Asset deleted successfully")).await;
        }
        Err(e) => {
            let _ = session.set_flash(FlashMessage::error(&format!("Failed to delete asset: {}", e))).await;
        }
    }

    // Redirect back to assets list
    Response::builder()
        .status(StatusCode::FOUND)
        .header("Location", "/assets")
        .body(axum::body::Body::empty())
        .unwrap()
}
