use axum::{
    extract::{Path, Query},
    response::{Html, Redirect},
    routing::get,
    routing::post,
    Form, Router,
};
use tower_sessions::{Session, SessionManagerLayer};
use async_session::MemoryStore;
use draftsmith_rest_api::client::{
    fetch_note, fetch_note_tree, notes::get_note_rendered_html, update_note, UpdateNoteRequest,
};
use include_dir::{include_dir, Dir, File};
use minijinja::{context, Environment, Error, ErrorKind};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use serde_json;

#[derive(Debug, Serialize, Deserialize, Clone)]
struct FlashMessage {
    kind: String,  // "success", "error", "info", "warning"
    message: String,
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

    // Add custom functions
    fn concat(a: String, b: String) -> Result<String, Error> {
        Ok(format!("{}{}", a, b))
    }

    fn menu_item(url: String, name: String) -> Result<String, Error> {
        Ok(format!(
            r#"
              <li>
                <!-- TODO -->
                <a href="{}" class="btn btn-ghost btn-sm justify-start">{}</a>
              </li>
                "#,
            url, name
        ))
    }

    env.add_function("c", concat);
    env.add_function("menu_item", menu_item);
    env
});

use crate::static_files::build_static_routes;

// TODO None Path should be 1
// TODO Better way than using a closure?
async fn route_note(session: Session, api_addr: String, Path(path): Path<i32>) -> Html<String> {
    let id = path;
    
    // Get and remove flash message
    let flash = session.remove::<FlashMessage>("flash").await.unwrap_or(None);

    // Get the note
    let note = fetch_note(&api_addr, id, true).await.unwrap_or_else(|e| {
        // TODO don't panic!
        panic!("Failed to fetch note. Error: {:#}", e);
    });

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
        flash => flash,
    )) {
        Ok(result) => result,
        Err(err) => handle_template_error(err),
    };

    Html(rendered)
}

async fn route_edit(api_addr: String, Path(path): Path<i32>) -> Html<String> {
    let id = path;
    // Get the note
    let note = fetch_note(&api_addr, id, false).await.unwrap_or_else(|e| {
        // TODO don't panic!
        panic!("Failed to fetch note. Error: {:#}", e);
    });

    // Load the template
    let template = ENV.get_template("body/note/edit.html").unwrap_or_else(|e| {
        panic!("Failed to load template. Error: {:#}", e);
    });

    let rendered = match template.render(context!(
    note => note,
    )) {
        Ok(result) => result,
        Err(err) => handle_template_error(err),
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
            session.insert("flash", FlashMessage {
                kind: "success".to_string(),
                message: "Note updated successfully".to_string(),
            }).await.unwrap();
        }
        Err(e) => {
            session.insert("flash", FlashMessage {
                kind: "error".to_string(),
                message: format!("Failed to update note: {}", e),
            }).await.unwrap();
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
    Html(format!("TODO Recent Pages"))
}

#[tokio::main]
pub async fn serve(api_scheme: &str, api_host: &str, api_port: &u16, host: &str, port: &str) {
    let api_addr = format!("{api_scheme}://{api_host}:{api_port}");
    let addr = format!("{}:{}", host, port);

    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .expect("Failed to bind address");

    // Create session store
    let session_store = MemoryStore::new();
    let session_layer = SessionManagerLayer::new(session_store)
        .with_secure(false);

    // Set up Routes
    let app = Router::new()
        .route(
            "/note/:id",
            get({
                let api_addr = api_addr.clone();
                move |session: Session, Path(path): Path<i32>| route_note(session, api_addr.clone(), Path(path))
            }),
        )
        .route(
            "/",
            get({
                let api_addr = api_addr.clone();
                move |session: Session| route_note(session, api_addr.clone(), Path(1))
            }),
        )
        .route(
            "/edit/:id",
            get({
                let api_addr = api_addr.clone();
                move |session: Session, Path(path): Path<i32>| route_edit(session, api_addr.clone(), Path(path))
            })
            .post({
                let api_addr = api_addr.clone();
                move |session: Session, Path(path): Path<i32>, Form(note): Form<UpdateNoteRequest>| {
                    route_update_note(session, api_addr.clone(), Path(path), Form(note))
                }
            }),
        )
        .nest("/static", build_static_routes())
        .route("/search", get(search))
        .route("/recent", get(recent))
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
