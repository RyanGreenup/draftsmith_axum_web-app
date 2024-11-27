use crate::flash::{FlashMessage, FlashMessageStore};
use axum::response::Redirect;
use include_dir::{include_dir, Dir};
use minijinja::Environment;
use minijinja::Error;
use once_cell::sync::Lazy;
use tower_sessions::Session;

static TEMPLATE_DIR: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/templates");

pub static ENV: Lazy<Environment<'static>> = Lazy::new(|| {
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

pub async fn handle_not_found(session: Session) -> Redirect {
    session
        .set_flash(FlashMessage::error("Page not found"))
        .await
        .unwrap_or_else(|e| {
            eprintln!("Failed to set flash message: {:#?}", e);
        });

    Redirect::to("/recent")
}

pub fn handle_template_error(err: Error) -> String {
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
