use axum::{
    extract::Path,
    response::{Html, IntoResponse},
    routing::{get, get_service},
    Router,
};
use minijinja::{context, path_loader, Environment};
use once_cell::sync::Lazy;
use std::collections::HashMap;

static ENV: Lazy<Environment<'static>> = Lazy::new(|| {
    let mut env = Environment::new();
    env.set_loader(path_loader("templates"));
    env
});

// Embed static files into the binary.
const INDEX_JS: &[u8] = include_bytes!("../static/index.js");
const STYLE_CSS: &[u8] = include_bytes!("../static/style.css");

#[tokio::main]
pub async fn serve(host: &str, port: &str) {
    // run our app with hyper, listening on specified host and port
    let addr = format!("{}:{}", host, port);
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();

    // Set up Routes
    let app = Router::new()
        .route("/", get(get_index))
        .nest_service("/static", get_static_files);

    // Do it!
    axum::serve(listener, app)
        .await
        .unwrap_or_else(|e| panic!("Unable to serve application. Error: {:#}", e));
}

async fn get_index() -> Html<String> {
    let temp_name = "index.html";
    let tmpl = ENV.get_template(temp_name).unwrap_or_else(|e| {
        panic!(
            "Unable to get template {:#} from environment. Error: {:#}",
            &temp_name, e
        )
    });
    let ctx = context!(name => "John", foo => "bar");
    let output = tmpl.render(ctx).unwrap_or_else(|e| {
        panic!("Unable to render template {:#}. Error: {:#}", &temp_name, e)
    });
    Html(output)
}

async fn get_static_files(Path(path): Path<String>) -> impl IntoResponse {
    match path.as_str() {
        "index.js" => ([(axum::http::header::CONTENT_TYPE, "application/javascript")], INDEX_JS),
        "style.css" => ([(axum::http::header::CONTENT_TYPE, "text/css")], STYLE_CSS),
        _ => (
            [(axum::http::header::CONTENT_TYPE, "text/plain")],
            b"Not Found",
        ),
    }
}
