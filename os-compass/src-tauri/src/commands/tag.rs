use crate::db::repository::TagRepository;
use crate::db::DbPool;
use crate::models::{Tag, CreateTagInput};
use tauri::State;

#[tauri::command]
pub fn get_tags(pool: State<'_, DbPool>) -> Result<Vec<Tag>, String> {
    let repo = TagRepository::new(&pool);
    repo.get_all()
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_project_tags(
    pool: State<'_, DbPool>,
    project_id: i64,
) -> Result<Vec<Tag>, String> {
    let repo = TagRepository::new(&pool);
    repo.get_by_project(project_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn create_tag(
    pool: State<'_, DbPool>,
    input: CreateTagInput,
) -> Result<i64, String> {
    let repo = TagRepository::new(&pool);
    let tag = Tag {
        id: 0,
        name: input.name,
        color: input.color,
        source: input.source.unwrap_or_else(|| "user".to_string()),
        created_at: String::new(),
    };
    repo.create(&tag)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn add_tag_to_project(
    pool: State<'_, DbPool>,
    project_id: i64,
    tag_id: i64,
) -> Result<(), String> {
    let repo = TagRepository::new(&pool);
    repo.add_to_project(project_id, tag_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn remove_tag_from_project(
    pool: State<'_, DbPool>,
    project_id: i64,
    tag_id: i64,
) -> Result<(), String> {
    let repo = TagRepository::new(&pool);
    repo.remove_from_project(project_id, tag_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_tag(
    pool: State<'_, DbPool>,
    id: i64,
) -> Result<(), String> {
    let repo = TagRepository::new(&pool);
    repo.delete(id)
        .map_err(|e| e.to_string())
}
