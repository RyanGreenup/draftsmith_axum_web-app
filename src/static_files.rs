use axum::http::StatusCode;
use axum::{extract::Path, http::header, response::IntoResponse, routing::get, Router};
use include_dir::{include_dir, Dir};

// Specify the directory you want to include
static CSS_DIR: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/static/css");
static JS_DIR: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/static/js");
static MEDIA_DIR: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/static/media");
static KATEX_DIR: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/static/katex");
static STIMULUS_DIR: Dir<'_> =
    include_dir!("$CARGO_MANIFEST_DIR/node_modules/@hotwired/stimulus/dist/");
static CONTROLLERS_DIR: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/static/js/controllers");

const NOT_FOUND_RESPONSE: &[u8] = b"Not Found";

// How to load a static file
// let admonitions = CSS_DIR.get_file("admonitions.css").unwrap();
// println!(
//     "Admonitions CSS: {:#?}",
//     String::from_utf8_lossy(admonitions.contents())
// );

pub fn build_static_routes() -> Router {
    Router::new()
        .route("/katex/dist/:path", get(get_static_katex_files))
        .route("/katex/dist/fonts/:path", get(get_static_katex_fonts))
        .route("/css/:path", get(get_static_css))
        .route("/js/:path", get(get_static_js))
        .route("/js/stimulus/:path", get(get_stimulus_js))
        .route("/js/controllers/:path", get(get_controllers))
        .route("/media/:path", get(get_static_media))
}

async fn get_static_katex_files(Path(path): Path<String>) -> impl IntoResponse {
    let not_found_string: &[u8] = b"Not Found";

    match path.as_str() {
        "katex.min.js" => (
            StatusCode::OK,
            [(axum::http::header::CONTENT_TYPE, "application/javascript")],
            KATEX_DIR.get_file("dist/katex.min.js").unwrap().contents(),
        ),
        "auto-render.min.js" => (
            StatusCode::OK,
            [(axum::http::header::CONTENT_TYPE, "application/javascript")],
            KATEX_DIR
                .get_file("dist/contrib/auto-render.min.js")
                .unwrap()
                .contents(),
        ),
        "katex.min.css" => (
            StatusCode::OK,
            [(axum::http::header::CONTENT_TYPE, "text/css")],
            KATEX_DIR.get_file("dist/katex.min.css").unwrap().contents(),
        ),
        _ => (
            StatusCode::NOT_FOUND,
            [(axum::http::header::CONTENT_TYPE, "text/plain")],
            not_found_string,
        ),
    }
}

async fn get_static_katex_fonts(Path(path): Path<String>) -> impl IntoResponse {
    let not_found_string: &[u8] = b"Not Found";

    // Check if the requested path corresponds to a font file
    if let Some(file) = KATEX_DIR.get_file(format!("dist/fonts/{}", path)) {
        let content_type = mime_guess::from_path(file.path())
            .first_or_octet_stream()
            .as_ref()
            .to_string();

        return (
            StatusCode::OK,
            [(axum::http::header::CONTENT_TYPE, content_type)],
            file.contents(),
        );
    }

    // Return 404 if the file is not found
    (
        StatusCode::NOT_FOUND,
        [(axum::http::header::CONTENT_TYPE, String::from("text/plain"))],
        not_found_string,
    )
}

async fn get_static_file(dir: &Dir<'_>, path: String) -> impl IntoResponse {
    // Try to get the file from the directory
    if let Some(file) = dir.get_file(&path) {
        let content_type = determine_content_type(file.path());
        return (
            StatusCode::OK,
            [(header::CONTENT_TYPE, content_type)],
            file.contents().to_vec(),
        );
    }

    // Return 404 if the file is not found
    (
        StatusCode::NOT_FOUND,
        [(header::CONTENT_TYPE, String::from("text/plain"))],
        NOT_FOUND_RESPONSE.to_vec(),
    )
}

fn determine_content_type(path: &std::path::Path) -> String {
    mime_guess::from_path(path)
        .first_or_octet_stream()
        .as_ref()
        .to_string()
}

async fn get_static_js(Path(path): Path<String>) -> impl IntoResponse {
    get_static_file(&JS_DIR, path).await
}

async fn get_stimulus_js(Path(path): Path<String>) -> impl IntoResponse {
    get_static_file(&STIMULUS_DIR, path).await
}

async fn get_static_media(Path(path): Path<String>) -> impl IntoResponse {
    get_static_file(&MEDIA_DIR, path).await
}

async fn get_static_css(Path(path): Path<String>) -> impl IntoResponse {
    get_static_file(&CSS_DIR, path).await
}

async fn get_controllers(Path(path): Path<String>) -> impl IntoResponse {
    get_static_file(&CONTROLLERS_DIR, path).await
}
