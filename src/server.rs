use crate::flash::{FlashMessage, FlashMessageStore};
use crate::html_builder::build_note_tree_html;
use crate::static_files::build_static_routes;
use axum::{
    extract::{Path, Query},
    response::{Html, Redirect},
    routing::{get, post},
    Form, Router,
};
use draftsmith_rest_api::client::{
    attach_child_note, detach_child_note, fetch_note, fetch_note_tree, get_note_breadcrumbs,
    notes::get_note_rendered_html, update_note, AttachChildRequest, NoteBreadcrumb,
    UpdateNoteRequest,
};
use include_dir::{include_dir, Dir};
use minijinja::{context, Environment, Error};
use once_cell::sync::Lazy;
use serde::Deserialize;
use tower_sessions::{MemoryStore, Session, SessionManagerLayer};

const MAX_ITEMS_PER_PAGE: usize = 3;

#[derive(Deserialize)]
struct PaginationParams {
    page: Option<i32>,
}

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

// TODO None Path should be 1
// TODO Better way than using a closure?
// TODO generalize these to inherit similar to the templates
async fn route_note(
    session: Session,
    api_addr: String,
    Path(path): Path<i32>,
    Query(params): Query<PaginationParams>,
) -> Html<String> {
    let id = path;

    // Get page from query params, defaulting to 1 if not present
    let current_page = params.page.unwrap_or(1).max(1);

    // Store current page in session
    session.insert("current_page", current_page).await.unwrap();

    // Get and remove flash message in one operation
    let flash = session.take_flash().await.unwrap_or(None);

    // Get the note
    let note = fetch_note(&api_addr, id, true).await.unwrap_or_else(|e| {
        // TODO don't panic!
        panic!("Failed to fetch note. Error: {:#}", e);
    });

    // Get the breadcrumbs
    let breadcrumbs: Option<Vec<NoteBreadcrumb>> = match get_note_breadcrumbs(&api_addr, id).await {
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
        Some(id),
        breadcrumbs
            .as_ref()
            .map_or_else(Vec::new, |b| b.iter().map(|bc| bc.id).collect()),
        MAX_ITEMS_PER_PAGE,
    );

    // Render the first note
    let rendered_note = get_note_rendered_html(&api_addr, id)
        .await
        .unwrap_or_else(|e| {
            panic!("Failed to get rendered note. Error: {:#}", e);
        });

    // Load the template
    let template = ENV.get_template("body/note/read.html").unwrap_or_else(|e| {
        panic!("Failed to load template. Error: {:#}", e);
    });

    let rendered = match template.render(context!(
        rendered_note => rendered_note,
        note => note,
        breadcrumbs => breadcrumbs,
        flash => flash,
        tree => tree,
        current_page => current_page,
        pages => tree,  // Add this to enable pagination template
    )) {
        Ok(result) => result,
        Err(err) => handle_template_error(err),
    };

    Html(rendered)
}

async fn route_edit(session: Session, api_addr: String, Path(path): Path<i32>) -> Html<String> {
    let id = path;

    // Get the note
    let note = match fetch_note(&api_addr, id, false).await {
        Ok(note) => note,
        Err(e) => {
            session
                .set_flash(FlashMessage::error(format!("Failed to fetch note: {}", e)))
                .await
                .unwrap();

            // Redirect to home page or another appropriate page when note fetch fails
            return Html(format!(
                r#"<script>window.location.href = "/note/{}";</script>"#,
                id
            ));
        }
    };

    // Get the tree and breadcrumbs
    let breadcrumbs: Option<Vec<NoteBreadcrumb>> = match get_note_breadcrumbs(&api_addr, id).await {
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
        Some(id),
        breadcrumbs
            .as_ref()
            .map_or_else(Vec::new, |b| b.iter().map(|bc| bc.id).collect()),
        MAX_ITEMS_PER_PAGE,
    );

    // Load the template
    let template = match ENV.get_template("body/note/edit.html") {
        Ok(template) => template,
        Err(e) => {
            session
                .set_flash(FlashMessage::error(format!(
                    "Failed to load template: {}",
                    e
                )))
                .await
                .unwrap();

            return Html(format!(
                r#"<script>window.location.href = "/note/{}";</script>"#,
                id
            ));
        }
    };

    let rendered = match template.render(context!(
        note => note,
        tree => tree,
        breadcrumbs => breadcrumbs,
    )) {
        Ok(result) => result,
        Err(err) => {
            session
                .set_flash(FlashMessage::error(format!(
                    "Failed to render template: {}",
                    err
                )))
                .await
                .unwrap();

            handle_template_error(err)
        }
    };

    Html(rendered)
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
async fn recent() -> Html<String> {
    Html("TODO Recent Pages".to_string())
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
                move |session: Session,
                      Path(path): Path<i32>,
                      query: Query<PaginationParams>| {
                    route_note(session, api_addr.clone(), Path(path), query)
                }
            }),
        )
        .route(
            "/",
            get({
                let api_addr = api_addr.clone();
                move |session: Session, query: Query<PaginationParams>| {
                    route_note(session, api_addr.clone(), Path(1), query)
                }
            }),
        )
        .route(
            "/edit/:id",
            get({
                let api_addr = api_addr.clone();
                move |session: Session, Path(path): Path<i32>| {
                    route_edit(session, api_addr.clone(), Path(path))
                }
            })
            .post({
                let api_addr = api_addr.clone();
                move |session: Session,
                      Path(path): Path<i32>,
                      Form(note): Form<UpdateNoteRequest>| {
                    route_update_note(session, api_addr.clone(), Path(path), Form(note))
                }
            }),
        )
        .nest("/static", build_static_routes())
        .route("/search", get(search))
        .route("/recent", get(recent))
        .route(
            "/note/:id/move",
            get({
                let api_addr = api_addr.clone();
                move |Path(path): Path<i32>| route_move_note_get(api_addr.clone(), Path(path))
            })
            .post({
                let api_addr = api_addr.clone();
                move |session: Session, Path(path): Path<i32>, Form(form): Form<MoveNoteForm>| {
                    route_move_note_post(session, api_addr.clone(), Path(path), Form(form))
                }
            }),
        )
        .route(
            "/note/:id/detach",
            post({
                let api_addr = api_addr.clone();
                move |session: Session, Path(path): Path<i32>| {
                    route_detach_note_post(session, api_addr.clone(), Path(path))
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
