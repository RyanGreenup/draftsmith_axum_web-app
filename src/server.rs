use axum::{response::Html, routing::get, Router};
use minijinja::{context, path_loader, Environment};
use once_cell::sync::Lazy;

static ENV: Lazy<Environment<'static>> = Lazy::new(|| {
    let mut env = Environment::new();
    env.set_loader(path_loader("templates"));
    env.add_template("index.html", include_str!("../templates/index.html"))
        .unwrap_or_else(|e| {
            panic!(
                "Unable to add template {:#} to environment. Error: {:#}",
                "index.html", e
            )
        });
    env.add_template("head.html", include_str!("../templates/head.html"))
        .unwrap_or_else(|e| {
            panic!(
                "Unable to add template {:#} to environment. Error: {:#}",
                "head.html", e
            )
        });
    env
});

use crate::static_files::get_static_files;

#[tokio::main]
pub async fn serve(host: &str, port: &str) {
    // run our app with hyper, listening on specified host and port
    let addr = format!("{}:{}", host, port);
    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .expect("Failed to bind address");

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
