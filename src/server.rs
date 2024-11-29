use crate::routes::{
    notes::{
        create::route_create,
        edit::{route_edit, route_update_note},
        note_move::{route_detach_note_post, route_move_note_get, route_move_note_post},
        view::route_note,
        delete::route_delete,
    },
    tags::{
        manage_all_tags::route_manage_tags,
        create::route_create_tag,
        delete::route_delete_tag,
        update::route_update_tag,
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

        .route("/note/:id", get(route_note))
        .route("/note/:id/delete", post(route_delete))
        .route(
            "/note/:id/move",
            get(route_move_note_get).post(route_move_note_post),
        )
        .route("/note/:id/detach", post(route_detach_note_post))
        .layer(CompressionLayer::new())
        .with_state(state)
        .layer(session_layer);

    // Do it!
    axum::serve(listener, app)
        .await
        .unwrap_or_else(|e| panic!("Unable to serve application. Error: {:#}", e));
}
