use axum::{extract::Path, extract::Query, response::Html, routing::get, Router};
use draftsmith_rest_api::client::{fetch_note, fetch_note_tree, notes::get_note_rendered_html};
use include_dir::{include_dir, Dir, File};
use minijinja::{context, Environment, Error, ErrorKind};
use once_cell::sync::Lazy;
use serde_json;

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
async fn render_index(api_addr: String, Path(path): Path<i32>) -> Html<String> {
    let id = path;
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
    // TODO don't panic
    let template = ENV.get_template("body/note/base.html").unwrap_or_else(|e| {
        panic!("Failed to load template. Error: {:#}", e);
    });

    // // Render the template
    // // // TODO clean these up
    // let rendered = template
    //     .render(context!(
    //     rendered_note => rendered_note,
    //     note => note,
    //     ))
    //     .unwrap_or_else(|e| {
    //         panic!("Failed to render template. Error: {:#}", e);
    //     });

    let rendered = match template.render(context!(
    rendered_note => rendered_note,
    note => note,
    )) {
        Ok(result) => result,
        Err(err) => {
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
    };

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

    // Set up Routes
    let app = Router::new()
        .route(
            "/note/:id",
            get({
                let api_addr = api_addr.clone();
                move |Path(path): Path<i32>| render_index(api_addr.clone(), Path(path))
            }),
        )
        .route(
            "/",
            get({
                let api_addr = api_addr.clone();
                move || render_index(api_addr.clone(), Path(1))
            }),
        )
        .nest("/static", build_static_routes())
        .route("/search", get(search))
        .route("/recent", get(recent));

    // Do it!
    axum::serve(listener, app)
        .await
        .unwrap_or_else(|e| panic!("Unable to serve application. Error: {:#}", e));
}
