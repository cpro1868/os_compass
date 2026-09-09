use crate::db::repository::NoteRepository;
use crate::db::DbPool;
use crate::models::{ProjectNote, CreateNoteInput, UpdateNoteInput};
use tauri::State;

#[tauri::command]
pub fn get_project_notes(
    pool: State<'_, DbPool>,
    project_id: i64,
) -> Result<Vec<ProjectNote>, String> {
    let repo = NoteRepository::new(&pool);
    repo.get_by_project(project_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn create_note(
    pool: State<'_, DbPool>,
    input: CreateNoteInput,
) -> Result<i64, String> {
    let repo = NoteRepository::new(&pool);
    let note = ProjectNote {
        id: 0,
        project_id: input.project_id,
        content: input.content,
        created_at: String::new(),
        updated_at: String::new(),
    };
    repo.create(&note)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn update_note(
    pool: State<'_, DbPool>,
    input: UpdateNoteInput,
) -> Result<(), String> {
    let repo = NoteRepository::new(&pool);
    let note = ProjectNote {
        id: input.id,
        project_id: 0,
        content: input.content,
        created_at: String::new(),
        updated_at: String::new(),
    };
    repo.update(&note)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_note(
    pool: State<'_, DbPool>,
    id: i64,
) -> Result<(), String> {
    let repo = NoteRepository::new(&pool);
    repo.delete(id)
        .map_err(|e| e.to_string())
}
