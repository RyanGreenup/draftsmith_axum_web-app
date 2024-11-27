use draftsmith_rest_api::client::NoteTreeNode;
use std::fmt::Write;

pub struct TreePage {
    content: String,
    item_count: usize,
}

pub fn build_note_tree_html(
    tree: Vec<NoteTreeNode>,
    current_note_id: Option<i32>,
    parent_ids: Vec<i32>,
    max_items_per_page: usize,
) -> Vec<String> {
    let mut pages = Vec::new();
    let mut current_page = TreePage {
        content: String::new(),
        item_count: 0,
    };

    if max_items_per_page == 0 {
        return Vec::new();
    }
    
    // Start first page
    write!(
        current_page.content,
        r#"<ul class="menu bg-base-200 rounded-box w-full md:w-56" data-controller="tree">"#
    ).unwrap();

    // Track ancestry for context when splitting pages
    let mut ancestry = Vec::new();
    
    for node in &tree {
        render_node_with_paging(
            node,
            current_note_id,
            &parent_ids,
            -1,
            4,
            &mut current_page,
            &mut pages,
            max_items_per_page,
            &mut ancestry,
        );
    }

    // Close final page if it has content
    if current_page.item_count > 0 {
        current_page.content.push_str("</ul>");
        pages.push(current_page.content);
    }

    pages
}

fn render_node_with_paging(
    node: &NoteTreeNode,
    current_note_id: Option<i32>,
    parent_ids: &[i32],
    levels_below_current: i32,
    max_levels_below: i32,
    current_page: &mut TreePage,
    pages: &mut Vec<String>,
    max_items: usize,
    ancestry: &mut Vec<NoteTreeNode>,
) {
    // Check if we need to start a new page
    if current_page.item_count >= max_items {
        // Finish current page
        current_page.content.push_str("</ul>");
        pages.push(std::mem::take(&mut current_page.content));
        
        // Start new page
        write!(
            current_page.content,
            r#"<ul class="menu bg-base-200 rounded-box w-full md:w-56" data-controller="tree">"#
        ).unwrap();
        
        // Reset count and add ancestry context
        current_page.item_count = 0;
        
        // Add ancestor nodes as context
        for ancestor in ancestry.iter() {
            render_context_node(current_page, ancestor, current_note_id, parent_ids);
        }
    }

    // Push current node to ancestry before processing children
    ancestry.push(node.clone());

    // Render current node
    render_single_node(current_page, node, current_note_id, parent_ids, levels_below_current);
    current_page.item_count += 1;

    // Process children if any
    if !node.children.is_empty() {
        current_page.content.push_str("<ul>");
        for child in &node.children {
            let next_level = if current_note_id == Some(node.id) {
                0
            } else if levels_below_current >= 0 {
                levels_below_current + 1
            } else {
                -1
            };

            render_node_with_paging(
                child,
                current_note_id,
                parent_ids,
                next_level,
                max_levels_below,
                current_page,
                pages,
                max_items,
                ancestry,
            );
        }
        current_page.content.push_str("</ul>");
    }

    // Remove current node from ancestry after processing
    ancestry.pop();
}

fn render_single_node(
    page: &mut TreePage,
    node: &NoteTreeNode,
    current_note_id: Option<i32>,
    parent_ids: &[i32],
    levels_below_current: i32,
) {
    let class_str = if Some(node.id) == current_note_id {
        r#"note-item bg-blue-100 text-blue-800 rounded-md"#
    } else {
        "note-item"
    };

    write!(
        page.content,
        r#"<li class="{}" draggable="true" data-note-id="{}">"#,
        class_str,
        node.id
    ).unwrap();

    let is_parent = parent_ids.contains(&node.id);
    let is_current = current_note_id == Some(node.id);
    
    write!(
        page.content,
        r#"<details{}"#,
        if is_parent || is_current { " open" } else { "" }
    ).unwrap();

    let title = node.title.as_deref().unwrap_or("Untitled");
    let summary_class = if Some(node.id) == current_note_id { "font-semibold" } else { "" };
    
    write!(
        page.content,
        r#"><summary class="{}"><a href="/note/{}">{}</a></summary></details></li>"#,
        summary_class,
        node.id,
        html_escape::encode_text(title)
    ).unwrap();
}

fn render_context_node(
    page: &mut TreePage,
    node: &NoteTreeNode,
    current_note_id: Option<i32>,
    parent_ids: &[i32],
) {
    write!(
        page.content,
        r#"<li class="note-item opacity-50" draggable="true" data-note-id="{}">"#,
        node.id
    ).unwrap();

    write!(
        page.content,
        r#"<details open><summary><a href="/note/{}">{}</a></summary></details></li>"#,
        node.id,
        html_escape::encode_text(node.title.as_deref().unwrap_or("Untitled"))
    ).unwrap();
}
