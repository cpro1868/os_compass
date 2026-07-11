use crate::db::repository::CategoryRepository;
use crate::db::DbPool;
use crate::models::{Category, CreateCategoryInput};
use tauri::State;

#[tauri::command]
pub fn get_categories(pool: State<'_, DbPool>) -> Result<Vec<Category>, String> {
    let repo = CategoryRepository::new(&pool);
    repo.get_all()
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_category(
    pool: State<'_, DbPool>,
    id: i64,
) -> Result<Option<Category>, String> {
    let repo = CategoryRepository::new(&pool);
    repo.get_by_id(id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn create_category(
    pool: State<'_, DbPool>,
    input: CreateCategoryInput,
) -> Result<i64, String> {
    let repo = CategoryRepository::new(&pool);
    let category = Category {
        id: 0,
        name: input.name,
        path: input.path,
        explain: input.explain,
        sort_order: input.sort_order.unwrap_or(0),
        parent_id: input.parent_id,
        is_system: false,
        created_at: String::new(),
    };
    repo.create(&category)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn update_category(
    pool: State<'_, DbPool>,
    category: Category,
) -> Result<(), String> {
    let repo = CategoryRepository::new(&pool);
    repo.update(&category)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_category(
    pool: State<'_, DbPool>,
    id: i64,
) -> Result<(), String> {
    let repo = CategoryRepository::new(&pool);
    repo.delete(id)
        .map_err(|e| e.to_string())
}
