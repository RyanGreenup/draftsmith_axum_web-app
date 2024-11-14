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
const KATEX_JS: &[u8] = include_bytes!("../static/katex/dist/katex.min.js");
const KATEX_AUTO_RENDER: &[u8] = include_bytes!("../static/katex/dist/contrib/auto-render.min.js");
const KATEX_CSS: &[u8] = include_bytes!("../static/katex/dist/katex.min.css");

#[tokio::main]
pub async fn serve(host: &str, port: &str) {
    // run our app with hyper, listening on specified host and port
    let addr = format!("{}:{}", host, port);
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();

    // Set up Routes
    let app = Router::new()
        .route("/", get(get_index))
        .route("/static/:path", get(get_static_files));

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
    let output = tmpl
        .render(ctx)
        .unwrap_or_else(|e| panic!("Unable to render template {:#}. Error: {:#}", &temp_name, e));
    Html(output)
}

async fn get_static_files(Path(path): Path<String>) -> impl IntoResponse {
    use axum::http::StatusCode;
    
    let not_found: &[u8] = b"Not Found";
    match path.as_str() {
        "katex/dist/katex.min.js" => (
            [(axum::http::header::CONTENT_TYPE, "application/javascript")],
            KATEX_JS,
        ),
        _ => (
            StatusCode::NOT_FOUND,
            [(axum::http::header::CONTENT_TYPE, "text/plain")],
            not_found,
        ),
    }
}
