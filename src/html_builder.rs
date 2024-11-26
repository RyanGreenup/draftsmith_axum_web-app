use std::fmt::Write;

pub fn build_note_tree_html(
    tree: &[Note],
    current_note_id: i64,
    parent_ids: &[i64]
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
    node: &Note,
    current_note_id: i64,
    parent_ids: &[i64]
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
            html_escape::encode_text(&node.title)
        ).unwrap();
    } else {
        write!(html, r#"><summary><a href="/note/{}">{}</a></summary>"#,
            node.id,
            html_escape::encode_text(&node.title)
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
