use crate::routes::{
    notes::{
        edit::{route_edit, route_update_note},
        note_move::{
            route_detach_note_post, route_move_note_get, route_move_note_post, MoveNoteForm,
        },
        view::route_note,
    },
    recent::route_recent,
    search::search,
};
use crate::static_files::build_static_routes;
use crate::template_context::PaginationParams;
use axum::{
    extract::{Path, Query},
    routing::{get, post},
    Form, Router,
};
use draftsmith_rest_api::client::UpdateNoteRequest;
use tower_sessions::{MemoryStore, Session, SessionManagerLayer};

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

    // Set up Routes
    let app = Router::new()
        .route(
            "/note/:id",
            get({
                let api_addr = api_addr.clone();
                move |session: Session, Path(id): Path<i32>, query: Query<PaginationParams>| async move {
                    route_note(session, api_addr.clone(), Path(id), query).await
                }
            }),
        )
        .route(
            "/",
            get({
                let api_addr = api_addr.clone();
                move |session: Session, query: Query<PaginationParams>| async move {
                    route_note(session, api_addr.clone(), Path(1), query).await
                }
            }),
        )
        .route(
            "/edit/:id",
            get({
                let api_addr = api_addr.clone();
                move |session: Session, Path(id): Path<i32>, query: Query<PaginationParams>| async move {
                    route_edit(session, api_addr.clone(), Path(id), query).await
                }
            })
            .post({
                let api_addr = api_addr.clone();
                move |session: Session, Path(id): Path<i32>, form: Form<UpdateNoteRequest>| async move {
                    route_update_note(session, api_addr.clone(), Path(id), form).await
                }
            }),
        )
        .nest("/static", build_static_routes())
        .route("/search", get(search))
        .route("/recent", get({
            let api_addr = api_addr.clone();
            move |session: Session, query: Query<PaginationParams>| async move {
                route_recent(session, query, api_addr.clone()).await
            }
        }))
        .route(
            "/note/:id/move",
            get({
                let api_addr = api_addr.clone();
                move |Path(id): Path<i32>| async move {
                    route_move_note_get(api_addr.clone(), Path(id)).await
                }
            })
            .post({
                let api_addr = api_addr.clone();
                move |session: Session, Path(id): Path<i32>, form: Form<MoveNoteForm>| async move {
                    route_move_note_post(session, api_addr.clone(), Path(id), form).await
                }
            }),
        )
        .route(
            "/note/:id/detach",
            post({
                let api_addr = api_addr.clone();
                move |session: Session, Path(id): Path<i32>| async move {
                    route_detach_note_post(session, api_addr.clone(), Path(id)).await
                }
            }),
        )
        .layer(session_layer);

    // Do it!
    axum::serve(listener, app)
        .await
        .unwrap_or_else(|e| panic!("Unable to serve application. Error: {:#}", e));
}

/*
{
    "/note":
        {
            "function": "route_note",
            "template": "body/note/read.html",
            "http_method": ["GET"],
        },
    "/edit":
        {
            "function": "route_edit",
            "template": "body/note/edit.html",
            "http_method": ["GET", "POST"]
        },
}
*/
