use std::fmt::Write;
use draftsmith_rest_api::client::NoteTreeNode;

pub fn build_note_tree_html(
    tree: Vec<NoteTreeNode>,
    current_note_id: i32,
    parent_ids: &[i32]
) -> String {
    let mut html = String::new();
    write!(html, r#"<ul class="menu bg-base-200 rounded-box w-full md:w-56">"#).unwrap();

    for node in tree {
        render_node(&mut html, node, current_note_id, parent_ids);
    }

    html.push_str("</ul>");
    html
}

fn render_node(
    html: &mut String,
    node: &NoteTreeNode,
    current_note_id: i32,
    parent_ids: &[i32]
) {
    // Start list item with conditional classes
    if node.id == current_note_id {
        write!(html, r#"<li class="note-item bg-blue-100 text-blue-800 rounded-md" draggable="true" data-note-id="{}">"#,
            node.id
        ).unwrap();
    } else {
        write!(html, r#"<li class="note-item" draggable="true" data-note-id="{}">"#,
            node.id
        ).unwrap();
    }

    // Details tag
    let is_open = parent_ids.contains(&node.id) || current_note_id == node.id;
    write!(html, r#"<details{}"#,
        if is_open { " open" } else { "" }
    ).unwrap();

    // Summary with conditional styling
    if node.id == current_note_id {
        write!(html, r#"><summary class="font-semibold"><a href="/note/{}">{}</a></summary>"#,
            node.id,
            html_escape::encode_text(&node.title.unwrap_or_else(|| "Untitled".to_string()))
        ).unwrap();
    } else {
        write!(html, r#"><summary><a href="/note/{}">{}</a></summary>"#,
            node.id,
            html_escape::encode_text(&node.title.as_ref().unwrap_or_else(|| &String::from("Untitled")))
        ).unwrap();
    }

    // Render children if any exist
    if !node.children.is_empty() {
        html.push_str("<ul>");
        for child in &node.children {
            render_node(html, child, current_note_id, parent_ids);
        }
        html.push_str("</ul>");
    }

    html.push_str("</details></li>");
}
