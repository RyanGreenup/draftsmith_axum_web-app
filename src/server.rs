use crate::flash::{FlashMessage, FlashMessageStore};
use crate::static_files::build_static_routes;
use axum::{
    extract::{Path, Query},
    response::{Html, Redirect},
    routing::get,
    Form, Router,
};
use draftsmith_rest_api::client::{
    fetch_note, get_note_breadcrumbs, notes::get_note_rendered_html, update_note, NoteBreadcrumb,
    UpdateNoteRequest, fetch_note_tree
};
use include_dir::{include_dir, Dir};
use minijinja::{context, Environment, Error};
use once_cell::sync::Lazy;
use tower_sessions::{MemoryStore, Session, SessionManagerLayer};
use crate::html_builder::build_note_tree_html;


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

// TODO None Path should be 1
// TODO Better way than using a closure?
// TODO generalize these to inherit similar to the templates
async fn route_note(session: Session, api_addr: String, Path(path): Path<i32>) -> Html<String> {
    let id = path;

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
    let tree = build_note_tree_html(tree, id, breadcrumbs.unwrap_or(Vec::new()).iter().map(|b| b.id).collect());


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
                move |session: Session, Path(path): Path<i32>| {
                    route_note(session, api_addr.clone(), Path(path))
                }
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
