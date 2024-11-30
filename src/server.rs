use axum::response::{IntoResponse, Response};
use axum::body::Body;
use axum::http::StatusCode;
use axum::http::header::{IF_NONE_MATCH, IF_MODIFIED_SINCE};
use reqwest::Client;
use axum::extract::Multipart;
use minijinja;
use chrono::{DateTime, Utc};
use tower_sessions::Session;
use crate::template;
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
    extract::{Path, DefaultBodyLimit},
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
        .route("/upload_asset", 
            get(route_upload_asset_form)
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
) -> impl IntoResponse {
    let template_name = "body/upload_asset.html";
    let tree_html = ""; // TODO: Implement sidebar tree html generation
    
    let context = minijinja::value::Value::from_serialize(&serde_json::json!({
        "tree_html": tree_html,
    }));

    template::render_template(template_name, context)
}

async fn route_upload_asset(
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> impl IntoResponse {
    let client = match Client::builder().build() {
        Ok(client) => client,
        Err(_) => {
            return Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(Body::from("Failed to initialize HTTP client"))
                .unwrap_or_else(|_| internal_server_error_response());
        }
    };

    let mut file_part = None;
    let mut location = None;

    while let Ok(Some(field)) = multipart.next_field().await {
        match field.name() {
            Some("file") => {
                file_part = Some(field);
            }
            Some("location") => {
                if let Ok(value) = field.text().await {
                    if !value.is_empty() {
                        location = Some(value);
                    }
                }
            }
            _ => continue,
        }
    }

    let file = match file_part {
        Some(file) => file,
        None => {
            return Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .body(Body::from("No file provided"))
                .unwrap_or_else(|_| internal_server_error_response());
        }
    };

    // Prepare the multipart request to the API
    let upload_url = format!("{}/assets/upload", state.api_addr);
    let filename = location.or_else(|| file.file_name().map(String::from));
    
    let mut form = reqwest::multipart::Form::new();
    
    // Add the file to the form
    if let Ok(data) = file.bytes().await {
        let part = reqwest::multipart::Part::bytes(data.to_vec())
            .file_name(filename.clone().unwrap_or_else(|| "file".to_string()));
        form = form.part("file", part);
    }

    if let Some(name) = filename {
        form = form.text("location", name);
    }

    match client.post(&upload_url)
        .multipart(form)
        .send()
        .await 
    {
        Ok(response) => {
            if response.status().is_success() {
                Response::builder()
                    .status(StatusCode::FOUND)
                    .header("Location", "/")  // Redirect to home page after successful upload
                    .body(Body::empty())
                    .unwrap_or_else(|_| internal_server_error_response())
            } else {
                Response::builder()
                    .status(response.status())
                    .body(Body::from("Failed to upload file"))
                    .unwrap_or_else(|_| internal_server_error_response())
            }
        }
        Err(_) => internal_server_error_response(),
    }
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
