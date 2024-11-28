use crate::flash::{FlashMessage, FlashMessageStore};
use crate::html_builder::build_note_tree_html;
use crate::routes::{
    notes::{
        edit::{route_edit, route_update_note},
        view::route_note,
    },
    recent::route_recent,
};
use crate::static_files::build_static_routes;
use crate::template_context::PaginationParams;
use crate::templates::{handle_template_error, ENV};
use axum::{
    extract::{Path, Query},
    response::{Html, Redirect},
    routing::{get, post},
    Form, Router,
};
use draftsmith_rest_api::client::{
    attach_child_note, detach_child_note, fetch_note_tree, get_note_breadcrumbs,
    AttachChildRequest, NoteBreadcrumb, UpdateNoteRequest,
};
use minijinja::context;
use serde::Deserialize;
use tower_sessions::{MemoryStore, Session, SessionManagerLayer};

const MAX_ITEMS_PER_PAGE: usize = 50;

// TODO implement search
async fn search(Query(params): Query<std::collections::HashMap<String, String>>) -> Html<String> {
    let search_term = params
        .get("q")
        .unwrap_or(&String::from("Unable to get Search Term"))
        .clone();
    Html(format!("Search term: {}", search_term))
}

#[derive(Deserialize)]
struct MoveNoteForm {
    new_parent_id: i32,
}

async fn route_move_note_get(api_addr: String, Path(note_id): Path<i32>) -> Html<String> {
    let template = ENV.get_template("body/note/move.html").unwrap();

    // Get the breadcrumbs
    let breadcrumbs: Option<Vec<NoteBreadcrumb>> =
        match get_note_breadcrumbs(&api_addr, note_id).await {
            Ok(b) => Some(b),
            Err(e) => {
                eprintln!("Failed to get Note Breadcrumbs: {:#?}", e);
                None
            }
        };

    // Get the Tree
    let tree = fetch_note_tree(&api_addr).await.unwrap_or_else(|e| {
        // TODO don't panic!
        panic!("Failed to fetch note tree. Error: {:#}", e);
    });
    let tree = build_note_tree_html(
        tree,
        Some(note_id),
        breadcrumbs
            .as_ref()
            .map_or_else(Vec::new, |b| b.iter().map(|bc| bc.id).collect()),
        MAX_ITEMS_PER_PAGE, // max items per page
    );

    let rendered = template
        .render(context!(
            note_id => note_id,
            tree => tree,
        ))
        .unwrap_or_else(handle_template_error);

    Html(rendered)
}

async fn route_detach_note_post(
    session: Session,
    api_addr: String,
    Path(note_id): Path<i32>,
) -> Redirect {
    match detach_child_note(&api_addr, note_id).await {
        Ok(_) => {
            session
                .set_flash(FlashMessage::success("Note detached successfully"))
                .await
                .unwrap();
        }
        Err(e) => {
            session
                .set_flash(FlashMessage::error(format!("Failed to detach note: {}", e)))
                .await
                .unwrap();
        }
    }

    Redirect::to(&format!("/note/{note_id}"))
}

async fn route_move_note_post(
    session: Session,
    api_addr: String,
    Path(note_id): Path<i32>,
    Form(form): Form<MoveNoteForm>,
) -> Redirect {
    // Get the breadcrumbs to check for parents
    let breadcrumbs: Option<Vec<NoteBreadcrumb>> =
        match get_note_breadcrumbs(&api_addr, note_id).await {
            Ok(b) => Some(b),
            Err(e) => {
                eprintln!("Failed to get Note Breadcrumbs: {:#?}", e);
                None
            }
        };

    // Detach the note from its current parent if it has one
    if let Some(bc) = breadcrumbs {
        if bc.len() > 1 {
            match detach_child_note(&api_addr, note_id).await {
                Ok(_) => (),
                Err(e) => {
                    session
                        .set_flash(FlashMessage::error(format!("Failed to detach note: {}", e)))
                        .await
                        .unwrap_or_else(|e| {
                            eprintln!("Failed to set flash message: {:#?}", e);
                        });
                    return Redirect::to(&format!("/note/{note_id}"));
                }
            }
        }
    }

    // NOTE attaching to id=0 will effectively detach
    // I am relying on this in ../static/js/controllers/tree_controller.js
    // Probably a candidate for a refactor
    // Then attach it to the new parent
    let attach_request = AttachChildRequest {
        parent_note_id: Some(form.new_parent_id),
        child_note_id: note_id,
    };

    // Flash the result
    match attach_child_note(&api_addr, attach_request).await {
        Ok(_) => {
            session
                .set_flash(FlashMessage::success("Note moved successfully"))
                .await
                .unwrap();
        }
        Err(e) => {
            session
                .set_flash(FlashMessage::error(format!("Failed to move note: {}", e)))
                .await
                .unwrap();
        }
    }

    Redirect::to(&format!("/note/{note_id}"))
}

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
