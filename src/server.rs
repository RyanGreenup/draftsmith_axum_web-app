use axum::{extract::Path, extract::Query, response::Html, routing::get, Router};
use draftsmith_rs_api::client::{fetch_note_tree, notes::get_note_rendered_html};
use include_dir::{include_dir, Dir};
use minijinja::{context, Environment};
use once_cell::sync::Lazy;
use serde_json;

static TEMPLATE_DIR: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/templates");

static ENV: Lazy<Environment<'static>> = Lazy::new(|| {
    let mut env = Environment::new();
    // env.set_loader(path_loader("templates"));

    // Loop over TEMPLATE_DIR
    for file in TEMPLATE_DIR.files() {
        let contents = String::from_utf8_lossy(file.contents()).to_string();
        env.add_template_owned(file.path().to_str().unwrap(), contents)
            .unwrap();
    }
    env
});

use crate::static_files::build_static_routes;

// TODO None Path should be 1
// TODO Better way than using a closure?
async fn render_index(api_addr: String, Path(path): Path<i32>) -> Html<String> {
    let id = path;
    // Get all notes
    let all_notes = fetch_note_tree(&api_addr).await.unwrap_or_else(|e| {
        panic!("Failed to fetch notes. Error: {:#}", e);
    });

    // Render the first note
    let rendered_note = get_note_rendered_html(&api_addr, id)
        .await
        .unwrap_or_else(|e| {
            panic!("Failed to get rendered note. Error: {:#}", e);
        });

    // Get the note with id={id}
    let first_note = all_notes
        .iter()
        .find(|note| note.id == id)
        .unwrap_or_else(|| {
            panic!("Failed to find note with id={id}");
        });

    // Get the content
    let content = first_note.content.as_ref().unwrap_or_else(|| {
        panic!("Failed to get content for note with id={id}");
    });

    // clean up all_notes as a json
    let all_notes = serde_json::to_string_pretty(&all_notes).unwrap_or_else(|e| {
        panic!("Failed to serialize notes. Error: {:#}", e);
    });

    // Load the template
    let template = ENV.get_template("base.html").unwrap_or_else(|e| {
        panic!("Failed to load template. Error: {:#}", e);
    });

    // Render the template
    let rendered = template
        .render(context!(
        all_notes => all_notes,
        content => content,
        rendered_note => rendered_note,
        ))
        .unwrap_or_else(|e| {
            panic!("Failed to render template. Error: {:#}", e);
        });
    Html(rendered)
}

// TODO implement search
async fn search(Query(params): Query<std::collections::HashMap<String, String>>) -> Html<String> {
    let search_term = params
        .get("q")
        .unwrap_or(&String::from("Unable to get Search Term"))
        .clone();
    Html(format!("Search term: {}", search_term))
}

#[tokio::main]
pub async fn serve(api_scheme: &str, api_host: &str, api_port: &u16, host: &str, port: &str) {
    let api_addr = format!("{api_scheme}://{api_host}:{api_port}");
    let addr = format!("{}:{}", host, port);

    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .expect("Failed to bind address");

    // Set up Routes
    let app = Router::new()
        .route(
            "/:id",
            get(move |Path(path): Path<i32>| render_index(api_addr.clone(), Path(path))),
        )
        .route(
            "/",
            get(move || render_index(api_addr.clone(), Path(1))),
        )
        .nest("/static", build_static_routes())
        .route("/search", get(search));

    // Do it!
    axum::serve(listener, app)
        .await
        .unwrap_or_else(|e| panic!("Unable to serve application. Error: {:#}", e));
}
