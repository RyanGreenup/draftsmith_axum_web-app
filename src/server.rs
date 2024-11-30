use axum::response::{IntoResponse, Response};
use axum::body::Body;
use axum::http::StatusCode;
use axum::http::header::{IF_NONE_MATCH, IF_MODIFIED_SINCE};
use reqwest::Client;
use chrono::{DateTime, Utc};
use crate::routes::{
    notes::{
        create::route_create,
        edit::{route_edit, route_update_note},
        note_move::{route_detach_note_post, route_move_note_get, route_move_note_post},
        view::route_note,
        delete::route_delete,
        tags::{route_assign_tags_get, route_assign_tags_post},
    },
    tags::{
        manage_all_tags::route_manage_tags,
        create::route_create_tag,
        delete::route_delete_tag,
        update::{route_update_tag, route_set_parent, route_unset_parent},
        list::route_list_tag,
    },
    recent::route_recent,
    search::search,
};
use crate::state::AppState;
use crate::static_files::build_static_routes;
use axum::extract::State;
use axum::{
    extract::Path,
    routing::{get, post},
    Router,
};
use tower_http::compression::CompressionLayer;
use tower_sessions::{MemoryStore, SessionManagerLayer};

#[tokio::main]
pub async fn serve(api_scheme: &str, api_host: &str, api_port: &u16, host: &str, port: &str) {
    let api_addr = format!("{api_scheme}://{api_host}:{api_port}");
    let addr = format!("{}:{}", host, port);

    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .expect("Failed to bind address");

    // Create session store
    let session_store = MemoryStore::default();
    let session_layer = SessionManagerLayer::new(session_store).with_secure(false);

    // Create shared state
    let state = AppState {
        api_addr: api_addr.clone(),
    };

    // Set up Routes
    let app = Router::<AppState>::new()
        .route(
            "/",
            get(|session, query, state: State<AppState>| {
                route_note(session, state, Path(1), query)
            }),
        )
        .route("/create", get(|session, state: State<AppState>, query| {
            route_create(session, state, Path(None), query)
        }))
        .route("/create/:id", get(|session, state: State<AppState>, Path(id): Path<i32>, query| {
            route_create(session, state, Path(Some(id)), query)
        }))
        .route("/edit/:id", get(route_edit).post(route_update_note))
        .nest("/static", build_static_routes())
        .route("/search", get(search))
        .route("/recent", get(route_recent))
        .route("/manage_tags", get(route_manage_tags))
        .route("/create_tag", post(route_create_tag))
        .route("/delete_tag/:id", post(route_delete_tag))
        .route("/rename_tag/:id", post(route_update_tag))
        .route("/tag/:id/set_parent", post(route_set_parent))
        .route("/tag/:id/unset_parent", post(route_unset_parent))
        .route("/tags/:id", get(route_list_tag))

        .route("/note/:id", get(route_note))
        .route("/note/:id/delete", get(route_delete))
        .route(
            "/note/:id/move",
            get(route_move_note_get).post(route_move_note_post),
        )
        .route("/note/:id/detach", post(route_detach_note_post))
        .route("/assign_tags/:id", get(route_assign_tags_get).post(route_assign_tags_post))
        .route("/m/*file_path", get(route_serve_asset))
        .layer(CompressionLayer::new())
        .with_state(state)
        .layer(session_layer);

async fn route_serve_asset(
    State(state): State<AppState>,
    Path(file_path): Path<String>,
    headers: axum::http::HeaderMap,
) -> Response {
    let client = Client::new();
    let asset_url = format!("{}/assets/download/{}", state.api_addr, file_path);

    // Forward conditional headers if present
    let mut request = client.get(&asset_url);
    if let Some(if_none_match) = headers.get(IF_NONE_MATCH) {
        request = request.header(IF_NONE_MATCH, if_none_match);
    }
    if let Some(if_modified_since) = headers.get(IF_MODIFIED_SINCE) {
        request = request.header(IF_MODIFIED_SINCE, if_modified_since);
    }

    match request.send().await {
        Ok(response) => {
            let status = response.status();
            let headers = response.headers().clone();

            // If API returns 304 Not Modified, return that directly
            if status == StatusCode::NOT_MODIFIED {
                return Response::builder()
                    .status(StatusCode::NOT_MODIFIED)
                    .body(Body::empty())
                    .unwrap_or_default();
            }

            let bytes = response.bytes().await.unwrap_or_default();
            
            let mut builder = Response::builder()
                .status(status)
                .header("content-type", headers.get("content-type")
                    .unwrap_or(&"application/octet-stream".parse().unwrap()));

            // Forward cache-related headers from API
            for header in ["cache-control", "etag", "last-modified"] {
                if let Some(value) = headers.get(header) {
                    builder = builder.header(header, value);
                }
            }

            // If API didn't provide cache headers, set reasonable defaults
            if !headers.contains_key("cache-control") {
                builder = builder.header("cache-control", "public, max-age=3600"); // Cache for 1 hour
            }
            if !headers.contains_key("etag") && !headers.contains_key("last-modified") {
                // Generate simple etag from content length and last few bytes
                let simple_etag = format!("W/\"{}-{}\"", 
                    bytes.len(),
                    bytes.iter().rev().take(4).map(|b| format!("{:02x}", b)).collect::<String>()
                );
                builder = builder.header("etag", simple_etag);
            }

            builder.body(Body::from(bytes))
                .unwrap_or_default()
        },
        Err(_) => Response::builder()
            .status(StatusCode::NOT_FOUND)
            .header("cache-control", "no-store")
            .header("content-type", "text/plain")
            .body(Body::from("Asset not found"))
            .unwrap_or_default()
    }
}

    // Do it!
    axum::serve(listener, app)
        .await
        .unwrap_or_else(|e| panic!("Unable to serve application. Error: {:#}", e));
}
