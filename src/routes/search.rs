use axum::{extract::Query, response::Html};

// TODO implement search
pub async fn search(
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> Html<String> {
    let search_term = params
        .get("q")
        .unwrap_or(&String::from("Unable to get Search Term"))
        .clone();
    Html(format!("Search term: {}", search_term))
}
