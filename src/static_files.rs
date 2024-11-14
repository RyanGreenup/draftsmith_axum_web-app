use axum::http::StatusCode;
use axum::{extract::Path, response::IntoResponse};
use include_dir::{include_dir, Dir};

// Embed static files into the binary.
// const KATEX_JS: &[u8] = include_bytes!("../static/katex/dist/katex.min.js");
// const KATEX_AUTO_RENDER: &[u8] = include_bytes!("../static/katex/dist/contrib/auto-render.min.js");
// const KATEX_CSS: &[u8] = include_bytes!("../static/katex/dist/katex.min.css");

// Specify the directory you want to include
static CSS_DIR: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/static/css");
static JS_DIR: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/static/js");
static MEDIA_DIR: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/static/katex");
static KATEX_DIR: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/static/katex");

// How to load a static file
// let admonitions = CSS_DIR.get_file("admonitions.css").unwrap();
// println!(
//     "Admonitions CSS: {:#?}",
//     String::from_utf8_lossy(admonitions.contents())
// );

pub async fn get_static_files(Path(path): Path<String>) -> impl IntoResponse {
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

pub async fn get_static_katex_fonts(Path(path): Path<String>) -> impl IntoResponse {
    let not_found_string: &[u8] = b"Not Found";

    // Check if the requested path corresponds to a font file
    if let Some(file) = KATEX_DIR.get_file(format!("dist/fonts/{}", path)) {
        let content_type = match file.path().extension().and_then(|ext| ext.to_str()) {
            Some("woff") => "font/woff",
            Some("woff2") => "font/woff2",
            Some("ttf") => "font/ttf",
            Some("otf") => "font/otf",
            _ => "application/octet-stream",
        };

        return (
            StatusCode::OK,
            [(axum::http::header::CONTENT_TYPE, content_type)],
            file.contents(),
        );
    }

    // Return 404 if the file is not found
    (
        StatusCode::NOT_FOUND,
        [(axum::http::header::CONTENT_TYPE, "text/plain")],
        not_found_string,
    )
}
