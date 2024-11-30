use axum::body::Body;
use axum::http::StatusCode;
use axum::http::header::{IF_NONE_MATCH, IF_MODIFIED_SINCE};
use reqwest::Client;
use minijinja;
use tempfile;
use chrono::{DateTime, Utc};
use tower_sessions::Session;
use crate::flash::{FlashMessage, FlashMessageStore};
use crate::templates::{self, ENV, handle_template_error};
use crate::routes::assets::{route_list_assets, route_delete_asset};
use crate::template_context::{BodyTemplateContext, PaginationParams};
use draftsmith_rest_api::client::assets::{list_assets, create_asset, update_asset, delete_asset};
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
use axum::{
    extract::{Path, DefaultBodyLimit, State, Multipart, Query},
    routing::{get, post},
    response::{Html, IntoResponse, Response},
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


    let max_body_size = 1024 * 1024 * 1024; // 1 GB
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
        .route("/assets", get(route_list_assets))
        .route("/asset/:id/delete", post(route_delete_asset))
        .route("/upload_asset",
            get(|session, state, query| route_upload_asset_form(session, state, query))
            .post(route_upload_asset)
        )
        .layer(CompressionLayer::new())
        .layer(DefaultBodyLimit::max(max_body_size))
        .with_state(state)
        .layer(session_layer);

async fn route_serve_asset(
    State(state): State<AppState>,
    Path(file_path): Path<String>,
    headers: axum::http::HeaderMap,
) -> Response {
    let client = match Client::builder().build() {
        Ok(client) => client,
        Err(_) => {
            return Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .header("cache-control", "no-store")
                .body(Body::from("Failed to initialize HTTP client"))
                .unwrap_or_else(|_| internal_server_error_response())
        }
    };

    let asset_url = format!("{}/assets/download/{}", state.api_addr, file_path);
    let mut request = client.get(&asset_url);

    // Forward conditional headers if present
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
                    .unwrap_or_else(|_| not_modified_response());
            }

            // Handle non-success status codes from upstream
            if !status.is_success() {
                return map_upstream_error(status);
            }

            match response.bytes().await {
                Ok(bytes) => {
                    let mut builder = Response::builder().status(status);

                    // Set content-type, defaulting to octet-stream if not provided
                    let content_type = headers
                        .get("content-type")
                        .and_then(|h| h.to_str().ok())
                        .unwrap_or("application/octet-stream");
                    builder = builder.header("content-type", content_type);

                    // Forward cache-related headers from API
                    for header in ["cache-control", "etag", "last-modified"] {
                        if let Some(value) = headers.get(header) {
                            if let Ok(value_str) = value.to_str() {
                                builder = builder.header(header, value_str);
                            }
                        }
                    }

                    // If API didn't provide cache headers, set reasonable defaults
                    if !headers.contains_key("cache-control") {
                        builder = builder.header("cache-control", "public, max-age=3600");
                    }
                    if !headers.contains_key("etag") && !headers.contains_key("last-modified") {
                        // Generate simple etag from content length and last few bytes
                        let simple_etag = format!(
                            "W/\"{}-{}\"",
                            bytes.len(),
                            bytes
                                .iter()
                                .rev()
                                .take(4)
                                .map(|b| format!("{:02x}", b))
                                .collect::<String>()
                        );
                        builder = builder.header("etag", simple_etag);
                    }

                    builder
                        .body(Body::from(bytes))
                        .unwrap_or_else(|_| internal_server_error_response())
                }
                Err(_) => internal_server_error_response(),
            }
        }
        Err(e) => {
            // Handle network-level errors
            if e.is_timeout() {
                gateway_timeout_response()
            } else if e.is_connect() {
                bad_gateway_response()
            } else {
                internal_server_error_response()
            }
        }
    }
}

// Helper functions for common responses
fn internal_server_error_response() -> Response {
    Response::builder()
        .status(StatusCode::INTERNAL_SERVER_ERROR)
        .header("cache-control", "no-store")
        .header("content-type", "text/plain")
        .body(Body::from("Internal server error"))
        .unwrap_or_default()
}

fn not_modified_response() -> Response {
    Response::builder()
        .status(StatusCode::NOT_MODIFIED)
        .body(Body::empty())
        .unwrap_or_default()
}

fn bad_gateway_response() -> Response {
    Response::builder()
        .status(StatusCode::BAD_GATEWAY)
        .header("cache-control", "no-store")
        .header("content-type", "text/plain")
        .body(Body::from("Bad gateway"))
        .unwrap_or_default()
}

fn gateway_timeout_response() -> Response {
    Response::builder()
        .status(StatusCode::GATEWAY_TIMEOUT)
        .header("cache-control", "no-store")
        .header("content-type", "text/plain")
        .body(Body::from("Gateway timeout"))
        .unwrap_or_default()
}

async fn route_upload_asset_form(
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


    let template = ENV.get_template("body/upload_asset.html").unwrap_or_else(|e| {
        panic!("Failed to load template. Error: {:#}", e);
    });

    // Use the context from body_handler
    let rendered = template.render(body_handler.ctx).unwrap_or_else(handle_template_error);

    Html(rendered)
}

async fn route_upload_asset(
    State(state): State<AppState>,
    session: Session,
    mut multipart: Multipart,
) -> impl IntoResponse {
    // Process multipart form
    let mut file_part = None;
    let mut custom_location = None;

    while let Ok(Some(field)) = multipart.next_field().await {
        match field.name() {
            Some("file") => {
                let filename = field.file_name().map(String::from);
                if let Ok(bytes) = field.bytes().await {
                    file_part = Some((filename, bytes));
                }
            }
            Some("location") => {
                if let Ok(value) = field.text().await {
                    if !value.is_empty() {
                        custom_location = Some(value);
                    }
                }
            }
            _ => continue,
        }
    }

    // Check if we got a file
    let (original_filename, file_bytes) = match file_part {
        Some((Some(name), bytes)) => (name, bytes),
        _ => {
            let _ = session.set_flash(FlashMessage::error("No file provided")).await;
            return Response::builder()
                .status(StatusCode::FOUND)
                .header("Location", "/upload_asset")
                .body(Body::empty())
                .unwrap_or_else(|_| internal_server_error_response());
        }
    };

    // Create a temporary file
    let mut temp_file = match tempfile::NamedTempFile::new() {
        Ok(file) => file,
        Err(e) => {
            let _ = session.set_flash(FlashMessage::error(&format!("Failed to create temporary file: {}", e))).await;
            return Response::builder()
                .status(StatusCode::FOUND)
                .header("Location", "/upload_asset")
                .body(Body::empty())
                .unwrap_or_else(|_| internal_server_error_response());
        }
    };

    // Write bytes to temporary file
    if let Err(e) = std::io::Write::write_all(&mut temp_file, &file_bytes) {
        let _ = session.set_flash(FlashMessage::error(&format!("Failed to write temporary file: {}", e))).await;
        return Response::builder()
            .status(StatusCode::FOUND)
            .header("Location", "/upload_asset")
            .body(Body::empty())
            .unwrap_or_else(|_| internal_server_error_response());
    }

    // Use the custom location if provided, otherwise use the original filename
    let final_location = custom_location.or(Some(original_filename));

    // Use create_asset function
    let result = create_asset(
        &state.api_addr,
        temp_file.path(),
        None, // no note_id
        None, // no description
        final_location, // either custom filename or original filename
    ).await;

    // The temporary file will be automatically deleted when temp_file is dropped
    // at the end of this function, no manual cleanup needed!

    // Handle the result
    match result {
        Ok(asset) => {
            let filename = asset.location.display().to_string();
            let markdown = format!("![{}](/m/{})", filename, filename);
            let success_msg = format!(
                "File uploaded successfully. asset_id: {}\nUse this markdown to embed the file:\n {}",
                asset.id,
                markdown
            );
            let _ = session.set_flash(FlashMessage::success(&success_msg)).await;
        }
        Err(e) => {
            let _ = session.set_flash(FlashMessage::error(&format!("Upload failed: {}", e))).await;
        }
    }

    Response::builder()
        .status(StatusCode::FOUND)
        .header("Location", "/upload_asset")
        .body(Body::empty())
        .unwrap_or_else(|_| internal_server_error_response())
}

fn map_upstream_error(status: StatusCode) -> Response {
    let (status, message) = match status {
        StatusCode::NOT_FOUND => (StatusCode::NOT_FOUND, "Asset not found"),
        StatusCode::FORBIDDEN => (StatusCode::FORBIDDEN, "Access denied"),
        _ => (
            StatusCode::BAD_GATEWAY,
            "Unexpected response from upstream server",
        ),
    };

    Response::builder()
        .status(status)
        .header("cache-control", "no-store")
        .header("content-type", "text/plain")
        .body(Body::from(message))
        .unwrap_or_else(|_| internal_server_error_response())
}

    // Do it!
    axum::serve(listener, app)
        .tcp_nodelay(true)
        .await
        .unwrap_or_else(|e| panic!("Unable to serve application. Error: {:#}", e));
}
