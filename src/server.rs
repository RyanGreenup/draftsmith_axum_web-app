use crate::flash::{FlashMessage, FlashMessageStore};
use crate::html_builder::build_note_tree_html;
use crate::static_files::build_static_routes;
use draftsmith_rest_api::client::notes::NoteWithoutFts;
use axum::{
    extract::{Path, Query},
    response::{Html, Redirect},
    routing::{get, post},
    Form, Router,
};
use draftsmith_rest_api::client::{
    attach_child_note, detach_child_note, fetch_note, fetch_note_tree, get_note_breadcrumbs,
    notes::{get_note_rendered_html, fetch_notes, NoteError}, update_note, AttachChildRequest, NoteBreadcrumb,
    UpdateNoteRequest,
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

#[derive(Clone)]
struct BodyHandler {
    tree: Vec<String>,
}

impl BodyHandler {
    async fn new(api_addr: String) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        // Get tree
        let tree = fetch_note_tree(&api_addr).await?;
        let tree_html = build_note_tree_html(
            tree.clone(),
            None,
            Vec::new(),
            MAX_ITEMS_PER_PAGE,
        );

        Ok(Self {
            tree: tree_html,
        })
    }
}

#[derive(Clone)]
struct NoteHandler {
    api_addr: String,
    tree: Vec<String>,
    breadcrumbs: Vec<NoteBreadcrumb>,
    note: NoteWithoutFts,
}

impl NoteHandler {
    async fn new(api_addr: String, note_id: i32) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        // Get body handler data
        let body_handler = BodyHandler::new(api_addr.clone()).await?;

        // Get breadcrumbs
        let breadcrumbs = match get_note_breadcrumbs(&api_addr, note_id).await {
            Ok(b) => b,
            Err(e) => {
                eprintln!("Failed to get Note Breadcrumbs: {:#?}", e);
                Vec::new()
            }
        };

        // Get note
        let note = fetch_note(&api_addr, note_id, true).await?;

        Ok(Self {
            api_addr,
            tree: body_handler.tree,
            breadcrumbs,
            note,
        })
    }

    async fn get_note_with_content(&self, id: i32) -> Result<NoteWithoutFts, NoteError> {
        fetch_note(&self.api_addr, id, false).await
    }

    async fn get_rendered_html(&self, id: i32) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        Ok(get_note_rendered_html(&self.api_addr, id).await?)
    }
}

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
fn find_page_for_note(tree_pages: &Vec<String>, note_id: Option<i32>) -> i32 {
    if let Some(note_id) = note_id {
        for (index, page) in tree_pages.iter().enumerate() {
            if page.contains(&format!("data-note-id=\"{}\"", note_id)) {
                return (index + 1) as i32;
            }
        }
    }
    1 // Default to first page if note not found
}

async fn route_note(
    session: Session,
    api_addr: String,
    Path(id): Path<i32>,
    Query(params): Query<PaginationParams>,
) -> Html<String> {
    // Get note data
    let note_handler = match NoteHandler::new(api_addr, id).await {
        Ok(data) => data,
        Err(e) => {
            eprintln!("Failed to get note data: {:#}", e);
            return Html(String::from("<h1>Error fetching note data</h1>"));
        }
    };
    let breadcrumbs = note_handler.breadcrumbs.clone();
    let tree_pages = note_handler.tree.clone();
    let note = note_handler.note.clone();

    // Get rendered HTML
    let rendered_note = match note_handler.get_rendered_html(id).await {
        Ok(html) => html,
        Err(e) => {
            eprintln!("Failed to get rendered note: {:#}", e);
            return Html(String::from("<h1>Error rendering note</h1>"));
        }
    };

    // Get page from query params if present, otherwise find the page containing the note
    let current_page = params
        .page
        .unwrap_or_else(|| find_page_for_note(&tree_pages, Some(id)));
    let current_page = current_page.max(1);

    // Store current page in session
    session.insert("current_page", current_page).await.unwrap();

    // Get and remove flash message
    let flash = session.take_flash().await.unwrap_or(None);

    // Load and render template
    let template = ENV.get_template("body/note/read.html").unwrap_or_else(|e| {
        panic!("Failed to load template. Error: {:#}", e);
    });

    let rendered = match template.render(context!(
        rendered_note => rendered_note,
        note => note,
        breadcrumbs => breadcrumbs,
        flash => flash,
        tree => tree_pages,
        current_page => current_page,
        pages => tree_pages
    )) {
        Ok(result) => result,
        Err(err) => handle_template_error(err),
    };

    Html(rendered)
}

async fn route_edit(
    session: Session,
    api_addr: String,
    Path(id): Path<i32>,
    Query(params): Query<PaginationParams>,
) -> Html<String> {
    // Get note data
    let note_handler = match NoteHandler::new(api_addr, id).await {
        Ok(data) => data,
        Err(e) => {
            eprintln!("Failed to get note data: {:#}", e);
            return Html(String::from("<h1>Error fetching note data</h1>"));
        }
    };
    let breadcrumbs = note_handler.breadcrumbs.clone();
    let tree_pages = note_handler.tree.clone();
    let note = match note_handler.get_note_with_content(id).await {
        Ok(data) => data,
        Err(e) => {
            eprintln!("Failed to get note data: {:#}", e);
            return Html(String::from("<h1>Error fetching note content</h1>"));
        }
    };

    // Load template
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

    // Get page from query params if present, otherwise find the page containing the note
    let current_page = params
        .page
        .unwrap_or_else(|| find_page_for_note(&tree_pages, Some(id)));
    let current_page = current_page.max(1);


    let rendered = match template.render(context!(
        note => note,
        tree => tree_pages,
        breadcrumbs => breadcrumbs,
        current_page => current_page,
        pages => tree_pages
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
async fn route_recent(
    api_addr: String,
    Query(params): Query<PaginationParams>
) -> Html<String> {
    // Get the body data
    let body_handler = match BodyHandler::new(api_addr.clone()).await {
        Ok(handler) => handler,
        Err(e) => {
            eprintln!("Failed to create body handler: {:#?}", e);
            return Html(String::from("<h1>Error getting page data</h1>"));
        }
    };

    let tree_pages = body_handler.tree.clone();

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


    // Get page from query params if present, otherwise find the page containing the note
    let current_page = params
        .page
        .unwrap_or_else(|| find_page_for_note(&tree_pages, None));
    let current_page = current_page.max(1);

    let rendered = template
        .render(context!(
            recent_notes => recent_notes,
            current_page => current_page,
            tree => tree_pages,
            pages => tree_pages,
        ))
        .unwrap_or_else(handle_template_error);

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
            move |query: Query<PaginationParams>| async move {
                route_recent(api_addr.clone(), query).await
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
