use crate::flash::{FlashMessage, FlashMessageStore};
use axum::response::Redirect;
use include_dir::{include_dir, Dir};
use minijinja::{Environment, Error, Value};
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

    fn format_datetime(value: Value) -> Result<String, Error> {
        if let Some(datetime_str) = value.as_str() {
            // Split at 'T' and process
            if let Some((date, time)) = datetime_str.split_once('T') {
                // Take first 5 chars of time (HH:MM)
                let time = time.get(0..5).unwrap_or("00:00");
                return Ok(format!("{} {}", date, time));
            }
        }
        
        // If anything fails, return the original value
        Ok(value.to_string())
    }

    env.add_filter("format_datetime", format_datetime);

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
