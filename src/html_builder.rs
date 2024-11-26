use draftsmith_rest_api::client::NoteTreeNode;
use std::fmt::Write;

pub fn build_note_tree_html(
    tree: Vec<NoteTreeNode>,
    current_note_id: Option<i32>,
    parent_ids: Vec<i32>,
) -> String {
    let mut html = String::new();
    write!(
        html,
        r#"<ul class="menu bg-base-200 rounded-box w-full md:w-56">"#
    )
    .unwrap();

    for node in &tree {
        render_node(&mut html, node, current_note_id, &parent_ids, -1, 4);
    }

    html.push_str("</ul>");
    html
}

fn render_node(
    html: &mut String,
    node: &NoteTreeNode,
    current_note_id: Option<i32>,
    parent_ids: &[i32],
    levels_below_current: i32,
    max_levels_below: i32,
) {
    // Start list item with conditional classes
    if Some(node.id) == current_note_id {
        write!(html, r#"<li class="note-item bg-blue-100 text-blue-800 rounded-md" draggable="true" data-note-id="{}">"#,
            node.id
        ).unwrap();
    } else {
        write!(
            html,
            r#"<li class="note-item" draggable="true" data-note-id="{}">"#,
            node.id
        )
        .unwrap();
    }

    // Details tag
    let is_parent = parent_ids.contains(&node.id);
    let is_current = current_note_id == Some(node.id);
    let is_within_unfold_levels =
        levels_below_current >= 0 && levels_below_current < max_levels_below;

    write!(
        html,
        r#"<details{}"#,
        if is_parent || is_current || is_within_unfold_levels {
            " open"
        } else {
            ""
        }
    )
    .unwrap();

    // Summary with conditional styling
    let title = node
        .title
        .as_ref()
        .map(String::as_str)
        .unwrap_or("Untitled");
    if Some(node.id) == current_note_id {
        write!(
            html,
            r#"><summary class="font-semibold"><a href="/note/{}">{}</a></summary>"#,
            node.id,
            html_escape::encode_text(title)
        )
        .unwrap();
    } else {
        write!(
            html,
            r#"><summary><a href="/note/{}">{}</a></summary>"#,
            node.id,
            html_escape::encode_text(title)
        )
        .unwrap();
    }

    // Render children if any exist
    if !node.children.is_empty() {
        html.push_str("<ul>");
        for child in &node.children {
            let next_level = if is_current {
                // Start counting levels below current note
                0
            } else if levels_below_current >= 0 {
                // Continue counting levels below current note
                levels_below_current + 1
            } else {
                // Not in the current note's subtree
                -1
            };

            render_node(
                html,
                child,
                current_note_id,
                parent_ids,
                next_level,
                max_levels_below,
            );
        }
        html.push_str("</ul>");
    }

    html.push_str("</details></li>");
}
