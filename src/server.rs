use crate::flash::{FlashMessage, FlashMessageStore};
use crate::html_builder::build_note_tree_html;
use crate::static_files::build_static_routes;
use crate::template_context::{BodyTemplateContext, NoteTemplateContext, PaginationParams};
use axum::{
    extract::{Path, Query},
    response::{Html, IntoResponse, Redirect, Response},
    routing::{get, post},
    Form, Router,
};
use draftsmith_rest_api::client::{
    attach_child_note, detach_child_note, fetch_note_tree, get_note_breadcrumbs,
    notes::fetch_notes, update_note, AttachChildRequest, NoteBreadcrumb, UpdateNoteRequest,
};
use include_dir::{include_dir, Dir};
use minijinja::{context, Environment, Error};
use once_cell::sync::Lazy;
use serde::Deserialize;
use tower_sessions::{MemoryStore, Session, SessionManagerLayer};

const MAX_ITEMS_PER_PAGE: usize = 50;

/*
- Body
    - Vars
        - api_addr: Str
        - tree: Vec<String>
        - breadcrumbs: Vec<NoteBreadcrumb>
    - Templates
        - body/base.html
        - body/pagination.html
        - body/recent.html
    - Sub
        - Notes
            - Vars
                - note: NoteWithoutFts
            - Templates
                - body/note/read.html
                - body/note/edit.html
                - body/note/move.html
*/

static TEMPLATE_DIR: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/templates");

static ENV: Lazy<Environment<'static>> = Lazy::new(|| {
    let mut env = Environment::new();
    // env.set_loader(path_loader("templates"));

    for entry in TEMPLATE_DIR
        .find("**/*.html")
        .expect("Unable to walk Template Directory")
    {
        if let Some(file) = entry.as_file() {
            let contents = String::from_utf8_lossy(file.contents()).to_string();
            env.add_template_owned(file.path().to_str().unwrap(), contents)
                .unwrap();
        }
    }

    /*

    // Example: Add custom functions
    fn concat(a: String, b: String) -> Result<String, Error> {
        Ok(format!("{}{}", a, b))
    }
    env.add_function("c", concat);

    */

    env
});

async fn route_note(
    session: Session,
    api_addr: String,
    Path(id): Path<i32>,
    Query(params): Query<PaginationParams>,
) -> Response {
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

async fn route_edit(
    session: Session,
    api_addr: String,
    Path(id): Path<i32>,
    Query(params): Query<PaginationParams>,
) -> Response {
    // Get note data
    let note_handler =
        match NoteTemplateContext::new(session.clone(), Query(params), api_addr, id).await {
            Ok(data) => data,
            Err(e) => {
                eprintln!("Failed to get note data: {:#}", e);
                return handle_not_found(session).await.into_response();
            }
        };

    // Load template
    let template = ENV
        .get_template("body/note/edit.html")
        .unwrap_or_else(|e| panic!("Failed to load template. Error: {:#}", e));

    // Render the template
    let rendered = template
        .render(note_handler.ctx)
        .unwrap_or_else(handle_template_error);

    Html(rendered).into_response()
}

async fn route_update_note(
    session: Session,
    api_addr: String,
    Path(path): Path<i32>,
    Form(note): Form<UpdateNoteRequest>,
) -> Redirect {
    let id = path;

    match update_note(&api_addr, id, note).await {
        Ok(_) => {
            session
                .set_flash(FlashMessage::success("Note updated successfully"))
                .await
                .unwrap();
        }
        Err(e) => {
            session
                .set_flash(FlashMessage::error(format!("Failed to update note: {}", e)))
                .await
                .unwrap();
        }
    }

    Redirect::to(&format!("/note/{id}"))
}

async fn handle_not_found(session: Session) -> Redirect {
    session
        .set_flash(FlashMessage::error("Page not found"))
        .await
        .unwrap_or_else(|e| {
            eprintln!("Failed to set flash message: {:#?}", e);
        });

    Redirect::to("/recent")
}

fn handle_template_error(err: Error) -> String {
    eprintln!("Could not render template: {:#}", err);
    // render causes as well
    let mut err = &err as &dyn std::error::Error;
    while let Some(next_err) = err.source() {
        eprintln!();
        eprintln!("caused by: {:#}", next_err);
        err = next_err;
    }
    String::from("<h1>Error rendering Template</h1></br> See STDERR for more information")
}

// TODO implement search
async fn search(Query(params): Query<std::collections::HashMap<String, String>>) -> Html<String> {
    let search_term = params
        .get("q")
        .unwrap_or(&String::from("Unable to get Search Term"))
        .clone();
    Html(format!("Search term: {}", search_term))
}

// TODO implement recent
async fn route_recent(
    session: Session,
    Query(params): Query<PaginationParams>,
    api_addr: String,
) -> Html<String> {
    // Get the body data
    let body_handler =
        match BodyTemplateContext::new(session, Query(params), api_addr.clone(), None).await {
            Ok(handler) => handler,
            Err(e) => {
                eprintln!("Failed to create body handler: {:#?}", e);
                return Html(String::from("<h1>Error getting page data</h1>"));
            }
        };

    // Get Recent notes
    let metadata_only = true;
    let mut notes = match fetch_notes(&api_addr, metadata_only).await {
        Ok(notes) => notes,
        Err(e) => {
            eprintln!("Failed to fetch notes: {:#?}", e);
            return Html(String::from("<h1>Error fetching notes</h1>"));
        }
    };

    // Sort notes by updated_at
    notes.sort_by(|a, b| a.modified_at.cmp(&b.modified_at));

    // Include only the last 50 notes
    let recent_notes = notes.iter().rev().take(50).collect::<Vec<_>>();

    let template = ENV.get_template("body/recent.html").unwrap_or_else(|e| {
        panic!("Failed to load template. Error: {:#}", e);
    });

    // get the context vars
    let ctx = context! { ..body_handler.ctx, ..context! {
        recent_notes => recent_notes,
    }};

    let rendered = template.render(ctx).unwrap_or_else(handle_template_error);

    Html(rendered)
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
