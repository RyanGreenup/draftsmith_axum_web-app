use crate::flash::FlashMessageStore;
use crate::html_builder::build_note_tree_html;
use crate::MAX_ITEMS_PER_PAGE;
use axum::extract::Query;
use minijinja::Environment;
use std::collections::HashSet;
use draftsmith_rest_api::client::tags::{list_note_tags, get_tag, get_tag_tree, update_tag, UpdateTagRequest, TagTreeNode};
use draftsmith_rest_api::client::notes::{NoteWithoutFts, get_backlinks, get_forward_links};
use draftsmith_rest_api::client::{
    fetch_note, fetch_note_tree, get_note_breadcrumbs,
    notes::{get_note_rendered_html, NoteError},
};
use minijinja::context;
use serde::Deserialize;
use tower_sessions::Session;


#[derive(Debug, Deserialize)]
pub struct PaginationParams {
    pub page: Option<i32>,
}

fn find_page_for_note(tree_pages: &[String], note_id: Option<i32>) -> i32 {
    if let Some(note_id) = note_id {
        for (index, page) in tree_pages.iter().enumerate() {
            if page.contains(&format!("data-note-id=\"{}\"", note_id)) {
                return (index + 1) as i32;
            }
        }
    }
    1 // Default to first page if note not found
}

/*
- Body
    - Vars
        - api_addr: Str
        - tree: Vec<String>
        - breadcrumbs: Vec<NoteBreadcrumb>
    - Templates
        - body/base.html
        - body/pagination.html
        - body/recent.html
    - Sub
        - Notes
            - Vars
                - note: NoteWithoutFts
            - Templates
                - body/note/read.html
                - body/note/edit.html
                - body/note/move.html
*/

#[derive(Clone)]
pub struct BodyTemplateContext {
    pub ctx: minijinja::Value,
}

impl BodyTemplateContext {
    pub async fn new(
        session: Session,
        Query(params): Query<PaginationParams>,
        api_addr: String,
        id: Option<i32>,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        // Get tree
        let tree_pages = fetch_note_tree(&api_addr).await?;
        let tree_html =
            build_note_tree_html(tree_pages.clone(), id, Vec::new(), MAX_ITEMS_PER_PAGE);

        // Get any Flash
        let flash = session.take_flash().await.unwrap_or(None);

        // Get sidebar page number from query params if present, otherwise find the page containing the note
        let current_page = params
            .page
            .unwrap_or_else(|| find_page_for_note(&tree_html, id));

        // Store current page in session
        // TODO don't panic
        session
            .insert("current_page", current_page)
            .await
            .expect("Unable to store current page");

        // Get the tag tree
        let tag_tree = match get_tag_tree(&api_addr).await {
            Ok(tree) => tree,
            Err(e) => {
                eprintln!("Failed to get tag tree: {:#?}", e);
                Vec::new()
            }
        };

        Ok(Self {
            ctx: context!(
            tree => tree_html,
            pages => tree_pages,
            flash => flash,
            current_page => current_page,
            tag_tree => tag_tree,
                ),
        })
    }
}

#[derive(Clone)]
pub struct NoteTemplateContext {
    api_addr: String,
    pub ctx: minijinja::Value,
}

impl NoteTemplateContext {
    pub async fn new(
        session: Session,
        Query(params): Query<PaginationParams>,
        api_addr: String,
        note_id: i32,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        // Get body handler data
        let body_handler =
            BodyTemplateContext::new(session, Query(params), api_addr.clone(), Some(note_id))
                .await?;

        // Get breadcrumbs
        let breadcrumbs = match get_note_breadcrumbs(&api_addr, note_id).await {
            Ok(b) => b,
            Err(e) => {
                eprintln!("Failed to get Note Breadcrumbs: {:#?}", e);
                Vec::new()
            }
        };

        // Get the backlinks
        let backlinks = match get_backlinks(&api_addr, note_id).await {
            Ok(b) => b,
            Err(e) => {
                eprintln!("Failed to get backlinks: {:#?}", e);
                Vec::new()
            }
        };

        // Get the forward links
        let forward_links = match get_forward_links(&api_addr, note_id).await {
            Ok(b) => b,
            Err(e) => {
                eprintln!("Failed to get forward links: {:#?}", e);
                Vec::new()
            }
        };

        // Get the tags
        let note_tag_relations = match list_note_tags(&api_addr).await {
            Ok(t) => t,
            Err(e) => {
                eprintln!("Failed to get note-tag relations: {:#?}", e);
                Vec::new()
            }
        };

        // Filter for tags belonging to this note
        let note_tags = note_tag_relations
            .into_iter()
            .filter(|nt| nt.note_id == note_id);

        // Resolve tag IDs to actual tag objects
        let mut tags = Vec::new();
        for note_tag in note_tags {
            if let Ok(tag) = get_tag(&api_addr, note_tag.tag_id).await {
                tags.push(tag);
            }
        }

        // Get note
        // TODO currently this fetches the note content even if it's not required.
        // This could be refactored to reduce requests, however, care needs to be taken to keep
        // the code simple
        // May try leptos next and circle back, managing web requests
        // in an MPA is a bit more tricky than expected.
        let note = fetch_note(&api_addr, note_id, false).await?;


        let ctx = context! { ..body_handler.ctx, ..context! {
            note => note,
            breadcrumbs => breadcrumbs,
            forwardlinks => forward_links,
            backlinks => backlinks,
            tags => tags,
        }};

        Ok(Self { api_addr, ctx })
    }

    // TODO use this to set note_content in the template
    // rather than note.content
    // that way the code stays simple but it's not fetched for reading
    #[allow(dead_code)]
    pub async fn get_note_with_content(&self, id: i32) -> Result<NoteWithoutFts, NoteError> {
        fetch_note(&self.api_addr, id, false).await
    }

    async fn get_tag_notes(&self, tag_id: i32) -> Vec<NoteWithoutFts> {
        // Get note-tag relations
        match list_note_tags(&self.api_addr).await {
            Ok(note_tags) => {
                // Filter for relevant tags
                let relevant_note_tags: Vec<_> = note_tags
                    .into_iter()
                    .filter(|nt| nt.tag_id == tag_id)
                    .collect();

                // Get note details for each relevant tag
                let mut notes = Vec::new();
                for note_tag in relevant_note_tags {
                    if let Ok(note) = fetch_note(&self.api_addr, note_tag.note_id, false).await {
                        notes.push(note);
                    }
                }
                notes
            }
            Err(e) => {
                eprintln!("Failed to get note-tag relations: {:#?}", e);
                Vec::new()
            }
        }
    }

    pub async fn get_rendered_html(
        &self,
        id: i32,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        Ok(get_note_rendered_html(&self.api_addr, id).await?)
    }
}
