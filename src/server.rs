use axum::{response::Html, routing::get, Router};
use minijinja::{context, path_loader, Environment};
use once_cell::sync::Lazy;
use draftsmith_rs_api::client::fetch_note_tree;

static ENV: Lazy<Environment<'static>> = Lazy::new(|| {
    let mut env = Environment::new();
    env.set_loader(path_loader("templates"));
    env
});

use crate::static_files::build_static_routes;

async fn render_index(base_url: &str) -> Html<String> {
    // all_notes = fetch_note_tree(base_url);
    let template = ENV.get_template("index.html").unwrap_or_else(|e| {
        panic!("Failed to load template. Error: {:#}", e);
    });
    let rendered = template.render(context!()).unwrap_or_else(|e| {
        panic!("Failed to render template. Error: {:#}", e);
    });
    Html(rendered)
}

#[tokio::main]
pub async fn serve(host: &str, port: &str) {
    let base_url = "your_base_url_here"; // Define your base URL here
    let addr = format!("{}:{}", host, port);
    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .expect("Failed to bind address");

    // Set up Routes
    let app = Router::new()
        .route("/", get(move || render_index(base_url)))
        .nest("/static", build_static_routes());

    // Do it!
    axum::serve(listener, app)
        .await
        .unwrap_or_else(|e| panic!("Unable to serve application. Error: {:#}", e));
}
