use crate::db::DATABASE;
use crate::models::{Project, Category, Tag, ProjectNote, ProjectUserInfo};
use crate::crypto::{encrypt_string, decrypt_string};
use log;

#[tauri::command]
pub fn get_projects() -> Result<Vec<Project>, String> {
    let db = DATABASE.lock().map_err(|e| e.to_string())?;
    let db = db.as_ref().ok_or("Database not initialized")?;
    let conn = db.get_connection();
    let mut stmt = conn
        .prepare("SELECT id, name, url, source, source_id, description, languages, stars, forks, open_issues, license, homepage, latest_commit, latest_release, readme_content, readme_lang, health_score, ai_summary, ai_use_cases, ai_risks, ai_dependencies, translated_summary, translated_use_cases, translated_risks, translated_dependencies, translated_description, readme_translation, category_id, lifecycle_status, data_status, is_downloaded, local_path, runbook, archived_at, deleted_at, created_at, updated_at FROM projects WHERE data_status = 'ACTIVE' ORDER BY updated_at DESC")
        .map_err(|e| e.to_string())?;
    let projects = stmt
        .query_map([], |row| {
            Ok(Project {
                id: row.get(0)?,
                name: row.get(1)?,
                url: row.get(2)?,
                source: row.get(3)?,
                source_id: row.get(4)?,
                description: row.get(5)?,
                languages: row.get(6)?,
                stars: row.get(7)?,
                forks: row.get(8)?,
                open_issues: row.get(9)?,
                license: row.get(10)?,
                homepage: row.get(11)?,
                latest_commit: row.get(12)?,
                latest_release: row.get(13)?,
                readme_content: row.get(14)?,
                readme_lang: row.get(15)?,
                health_score: row.get(16)?,
                ai_summary: row.get(17)?,
                ai_use_cases: row.get(18)?,
                ai_risks: row.get(19)?,
                ai_dependencies: row.get(20)?,
                translated_summary: row.get(21)?,
                translated_use_cases: row.get(22)?,
                translated_risks: row.get(23)?,
                translated_dependencies: row.get(24)?,
                translated_description: row.get(25)?,
                readme_translation: row.get(26)?,
                category_id: row.get(27)?,
                lifecycle_status: row.get(28)?,
                data_status: row.get(29)?,
                is_downloaded: row.get::<_, i32>(30)? != 0,
                local_path: row.get(31)?,
                runbook: row.get(32)?,
                archived_at: row.get(33)?,
                deleted_at: row.get(34)?,
                created_at: row.get(35)?,
                updated_at: row.get(36)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;
    Ok(projects)
}

#[tauri::command]
pub fn get_project(id: i64) -> Result<Option<Project>, String> {
    let db = DATABASE.lock().map_err(|e| e.to_string())?;
    let db = db.as_ref().ok_or("Database not initialized")?;
    let conn = db.get_connection();
    let mut stmt = conn
        .prepare("SELECT id, name, url, source, source_id, description, languages, stars, forks, open_issues, license, homepage, latest_commit, latest_release, readme_content, readme_lang, health_score, ai_summary, ai_use_cases, ai_risks, ai_dependencies, translated_summary, translated_use_cases, translated_risks, translated_dependencies, translated_description, readme_translation, category_id, lifecycle_status, data_status, is_downloaded, local_path, runbook, archived_at, deleted_at, created_at, updated_at FROM projects WHERE id = ?")
        .map_err(|e| e.to_string())?;
    let project = stmt
        .query_row([id], |row| {
            let p = Project {
                id: row.get(0)?,
                name: row.get(1)?,
                url: row.get(2)?,
                source: row.get(3)?,
                source_id: row.get(4)?,
                description: row.get(5)?,
                languages: row.get(6)?,
                stars: row.get(7)?,
                forks: row.get(8)?,
                open_issues: row.get(9)?,
                license: row.get(10)?,
                homepage: row.get(11)?,
                latest_commit: row.get(12)?,
                latest_release: row.get(13)?,
                readme_content: row.get(14)?,
                readme_lang: row.get(15)?,
                health_score: row.get(16)?,
                ai_summary: row.get(17)?,
                ai_use_cases: row.get(18)?,
                ai_risks: row.get(19)?,
                ai_dependencies: row.get(20)?,
                translated_summary: row.get(21)?,
                translated_use_cases: row.get(22)?,
                translated_risks: row.get(23)?,
                translated_dependencies: row.get(24)?,
                translated_description: row.get(25)?,
                readme_translation: row.get(26)?,
                category_id: row.get(27)?,
                lifecycle_status: row.get(28)?,
                data_status: row.get(29)?,
                is_downloaded: row.get::<_, i32>(30)? != 0,
                local_path: row.get(31)?,
                runbook: row.get(32)?,
                archived_at: row.get(33)?,
                deleted_at: row.get(34)?,
                created_at: row.get(35)?,
                updated_at: row.get(36)?,
            };
            Ok(p)
        })
        .ok();

    if let Some(ref p) = project {
        log::debug!("[GET_PROJECT] Project {}: translated_summary={:?}, translated_use_cases={:?}, translated_risks={:?}, translated_dependencies={:?}, translated_description={:?}",
            p.id, p.translated_summary, p.translated_use_cases, p.translated_risks, p.translated_dependencies, p.translated_description);
    }

    Ok(project)
}

#[tauri::command]
pub fn create_project(input: serde_json::Value) -> Result<i64, String> {
    let db = DATABASE.lock().map_err(|e| e.to_string())?;
    let db = db.as_ref().ok_or("Database not initialized")?;
    let conn = db.get_connection();
    let name: String = serde_json::from_value(input.get("name").unwrap_or(&serde_json::Value::Null).clone()).unwrap_or_default();
    let url: Option<String> = serde_json::from_value(input.get("url").unwrap_or(&serde_json::Value::Null).clone()).ok();
    let source: String = serde_json::from_value(input.get("source").unwrap_or(&serde_json::json!("manual")).clone()).unwrap_or_default();
    let source_id: Option<String> = serde_json::from_value(input.get("source_id").unwrap_or(&serde_json::Value::Null).clone()).ok();
    let description: Option<String> = serde_json::from_value(input.get("description").unwrap_or(&serde_json::Value::Null).clone()).ok();
    conn.execute(
        "INSERT INTO projects (name, url, source, source_id, description, created_at, updated_at) VALUES (?, ?, ?, ?, ?, datetime('now', 'localtime'), datetime('now', 'localtime'))",
        rusqlite::params![name, url, source, source_id, description],
    )
    .map_err(|e| e.to_string())?;
    Ok(conn.last_insert_rowid())
}

#[tauri::command]
pub fn update_project(input: serde_json::Value) -> Result<(), String> {
    let db = DATABASE.lock().map_err(|e| e.to_string())?;
    let db = db.as_ref().ok_or("Database not initialized")?;
    let conn = db.get_connection();
    let id: i64 = serde_json::from_value(input.get("id").unwrap_or(&serde_json::Value::Null).clone()).map_err(|e| e.to_string())?;
    let name: Option<String> = serde_json::from_value(input.get("name").unwrap_or(&serde_json::Value::Null).clone()).ok();
    let description: Option<String> = serde_json::from_value(input.get("description").unwrap_or(&serde_json::Value::Null).clone()).ok();
    let has_category = input.get("category_id").is_some();
    let category_id: Option<i64> = if has_category {
        let raw = input.get("category_id").unwrap();
        if raw.is_null() { None } else { serde_json::from_value(raw.clone()).ok() }
    } else {
        None
    };

    if let Some(ref n) = name {
        let trimmed = n.trim();
        if trimmed.is_empty() {
            return Err("项目名称不能为空".to_string());
        }
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM projects WHERE name = ? AND id != ? AND data_status = 'ACTIVE'",
            rusqlite::params![trimmed, id],
            |row| row.get(0),
        ).map_err(|e| e.to_string())?;
        if count > 0 {
            return Err(format!("项目名称 \"{}\" 已存在，请使用其他名称", trimmed));
        }
    }

    if has_category {
        let cat_value: Option<i64> = category_id;
        conn.execute(
            "UPDATE projects SET name = COALESCE(?, name), description = COALESCE(?, description), category_id = ?, updated_at = datetime('now', 'localtime') WHERE id = ?",
            rusqlite::params![name, description, cat_value, id],
        )
        .map_err(|e| e.to_string())?;
    } else {
        conn.execute(
            "UPDATE projects SET name = COALESCE(?, name), description = COALESCE(?, description), updated_at = datetime('now', 'localtime') WHERE id = ?",
            rusqlite::params![name, description, id],
        )
        .map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub fn update_project_status(id: i64, lifecycle_status: String) -> Result<(), String> {
    let db = DATABASE.lock().map_err(|e| e.to_string())?;
    let db = db.as_ref().ok_or("Database not initialized")?;
    let conn = db.get_connection();
    conn.execute(
        "UPDATE projects SET lifecycle_status = ?, updated_at = datetime('now', 'localtime') WHERE id = ?",
        rusqlite::params![lifecycle_status, id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn save_runbook(id: i64, content: String) -> Result<(), String> {
    let db = DATABASE.lock().map_err(|e| e.to_string())?;
    let db = db.as_ref().ok_or("Database not initialized")?;
    let conn = db.get_connection();
    conn.execute(
        "UPDATE projects SET runbook = ?, updated_at = datetime('now', 'localtime') WHERE id = ?",
        rusqlite::params![content, id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn get_runbook(id: i64) -> Result<Option<String>, String> {
    let db = DATABASE.lock().map_err(|e| e.to_string())?;
    let db = db.as_ref().ok_or("Database not initialized")?;
    let conn = db.get_connection();
    let mut stmt = conn
        .prepare("SELECT runbook FROM projects WHERE id = ?")
        .map_err(|e| e.to_string())?;
    let result = stmt.query_row([id], |row| row.get::<_, Option<String>>(0))
        .ok();
    Ok(result.flatten())
}

#[tauri::command]
pub fn delete_project(id: i64, permanent: bool) -> Result<(), String> {
    let db = DATABASE.lock().map_err(|e| e.to_string())?;
    let db = db.as_ref().ok_or("Database not initialized")?;
    let conn = db.get_connection();
    if permanent {
        conn.execute("DELETE FROM projects WHERE id = ?", [id])
    } else {
        conn.execute(
            "UPDATE projects SET data_status = 'DELETED', deleted_at = datetime('now', 'localtime') WHERE id = ?",
            [id],
        )
    }
    .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn get_categories() -> Result<Vec<Category>, String> {
    let db = DATABASE.lock().map_err(|e| e.to_string())?;
    let db = db.as_ref().ok_or("Database not initialized")?;
    let conn = db.get_connection();
    let mut stmt = conn
        .prepare(
            "SELECT c.id, c.name, c.parent_id, c.sort_order, c.created_at, c.updated_at, \
             (SELECT COUNT(*) FROM projects p WHERE p.category_id = c.id AND p.data_status = 'ACTIVE') \
             FROM categories c ORDER BY c.sort_order, c.name",
        )
        .map_err(|e| e.to_string())?;
    let categories = stmt
        .query_map([], |row| {
            let id: i64 = row.get(0)?;
            Ok(Category {
                id,
                name: row.get(1)?,
                path: format!("/{}", row.get::<_, String>(1).unwrap_or_default()),
                explain: None,
                sort_order: row.get(3)?,
                parent_id: row.get(2)?,
                is_system: id == 1,
                project_count: row.get(6)?,
                created_at: row.get(4)?,
                updated_at: row.get(5)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;
    Ok(categories)
}

#[tauri::command]
pub fn get_recent_projects(limit: i64) -> Result<Vec<Project>, String> {
    let db = DATABASE.lock().map_err(|e| e.to_string())?;
    let db = db.as_ref().ok_or("Database not initialized")?;
    let conn = db.get_connection();
    let mut stmt = conn
        .prepare(
            "SELECT id, name, url, source, source_id, description, languages, stars, forks, open_issues, license, homepage, latest_commit, latest_release, readme_content, readme_lang, health_score, ai_summary, ai_use_cases, ai_risks, ai_dependencies, translated_summary, translated_use_cases, translated_risks, translated_dependencies, translated_description, readme_translation, category_id, lifecycle_status, data_status, is_downloaded, local_path, runbook, archived_at, deleted_at, created_at, updated_at FROM projects WHERE data_status = 'ACTIVE' ORDER BY updated_at DESC LIMIT ?"
        )
        .map_err(|e| e.to_string())?;
    let projects = stmt
        .query_map([limit], |row| {
            Ok(Project {
                id: row.get(0)?,
                name: row.get(1)?,
                url: row.get(2)?,
                source: row.get(3)?,
                source_id: row.get(4)?,
                description: row.get(5)?,
                languages: row.get(6)?,
                stars: row.get(7)?,
                forks: row.get(8)?,
                open_issues: row.get(9)?,
                license: row.get(10)?,
                homepage: row.get(11)?,
                latest_commit: row.get(12)?,
                latest_release: row.get(13)?,
                readme_content: row.get(14)?,
                readme_lang: row.get(15)?,
                health_score: row.get(16)?,
                ai_summary: row.get(17)?,
                ai_use_cases: row.get(18)?,
                ai_risks: row.get(19)?,
                ai_dependencies: row.get(20)?,
                translated_summary: row.get(21)?,
                translated_use_cases: row.get(22)?,
                translated_risks: row.get(23)?,
                translated_dependencies: row.get(24)?,
                translated_description: row.get(25)?,
                readme_translation: row.get(26)?,
                category_id: row.get(27)?,
                lifecycle_status: row.get(28)?,
                data_status: row.get(29)?,
                is_downloaded: row.get::<_, i32>(30)? != 0,
                local_path: row.get(31)?,
                runbook: row.get(32)?,
                archived_at: row.get(33)?,
                deleted_at: row.get(34)?,
                created_at: row.get(35)?,
                updated_at: row.get(36)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;
    Ok(projects)
}

#[tauri::command]
pub fn get_project_preview(url: String) -> Result<serde_json::Value, String> {
    Ok(serde_json::json!({
        "name": "",
        "stars": null,
        "language": null,
    }))
}

#[tauri::command]
pub fn get_project_user_info(project_id: i64) -> Result<Vec<ProjectUserInfo>, String> {
    let db = DATABASE.lock().map_err(|e| e.to_string())?;
    let db = db.as_ref().ok_or("Database not initialized")?;
    let conn = db.get_connection();
    let mut stmt = conn
        .prepare("SELECT id, project_id, info_key, info_value, is_secret, created_at, updated_at FROM project_user_info WHERE project_id = ?")
        .map_err(|e| e.to_string())?;
    let infos = stmt
        .query_map([project_id], |row| {
            Ok(ProjectUserInfo {
                id: row.get(0)?,
                project_id: row.get(1)?,
                info_key: row.get(2)?,
                info_value: row.get(3)?,
                is_secret: row.get::<_, i32>(4)? != 0,
                created_at: row.get(5)?,
                updated_at: row.get(6)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;
    Ok(infos)
}

#[tauri::command]
pub fn add_project_user_info(
    project_id: i64,
    key: String,
    value: String,
    is_secret: bool,
    remark: Option<String>,
) -> Result<i64, String> {
    let db = DATABASE.lock().map_err(|e| e.to_string())?;
    let db = db.as_ref().ok_or("Database not initialized")?;
    let conn = db.get_connection();
    
    let store_value = if is_secret {
        encrypt_string(&value).map_err(|e| e.to_string())?
    } else {
        value
    };
    
    conn.execute(
        "INSERT INTO project_user_info (project_id, info_key, info_value, is_secret) VALUES (?, ?, ?, ?)",
        rusqlite::params![project_id, key, store_value, is_secret as i32],
    )
    .map_err(|e| e.to_string())?;
    
    Ok(conn.last_insert_rowid())
}

#[tauri::command]
pub fn update_project_user_info(
    id: i64,
    key: String,
    value: String,
    is_secret: bool,
    remark: Option<String>,
) -> Result<(), String> {
    let db = DATABASE.lock().map_err(|e| e.to_string())?;
    let db = db.as_ref().ok_or("Database not initialized")?;
    let conn = db.get_connection();
    
    let store_value = if is_secret {
        encrypt_string(&value).map_err(|e| e.to_string())?
    } else {
        value
    };
    
    conn.execute(
        "UPDATE project_user_info SET info_key = ?, info_value = ?, is_secret = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?",
        rusqlite::params![key, store_value, is_secret as i32, id],
    )
    .map_err(|e| e.to_string())?;
    
    Ok(())
}

#[tauri::command]
pub fn delete_project_user_info(id: i64) -> Result<(), String> {
    let db = DATABASE.lock().map_err(|e| e.to_string())?;
    let db = db.as_ref().ok_or("Database not initialized")?;
    let conn = db.get_connection();
    conn.execute("DELETE FROM project_user_info WHERE id = ?", [id])
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn copy_user_info_value(id: i64) -> Result<String, String> {
    let db = DATABASE.lock().map_err(|e| e.to_string())?;
    let db = db.as_ref().ok_or("Database not initialized")?;
    let conn = db.get_connection();
    
    let result = conn.query_row(
        "SELECT info_value, is_secret FROM project_user_info WHERE id = ?",
        [id],
        |row| {
            let value: String = row.get(0)?;
            let is_secret: i32 = row.get(1)?;
            Ok((value, is_secret))
        },
    ).map_err(|e| e.to_string())?;
    
    if result.1 != 0 {
        decrypt_string(&result.0).map_err(|e| e.to_string())
    } else {
        Ok(result.0)
    }
}

#[tauri::command]
pub fn batch_archive_projects(ids: Vec<i64>) -> Result<(), String> {
    let db = DATABASE.lock().map_err(|e| e.to_string())?;
    let db = db.as_ref().ok_or("Database not initialized")?;
    let conn = db.get_connection();
    let ids_str = ids.iter().map(|i| i.to_string()).collect::<Vec<_>>().join(",");
    if !ids_str.is_empty() {
        conn.execute(
            &format!("UPDATE projects SET data_status = 'ARCHIVED', archived_at = datetime('now', 'localtime'), updated_at = datetime('now', 'localtime') WHERE id IN ({})", ids_str),
            [],
        )
        .map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub fn batch_delete_projects(ids: Vec<i64>) -> Result<(), String> {
    let db = DATABASE.lock().map_err(|e| e.to_string())?;
    let db = db.as_ref().ok_or("Database not initialized")?;
    let conn = db.get_connection();
    let ids_str = ids.iter().map(|i| i.to_string()).collect::<Vec<_>>().join(",");
    if !ids_str.is_empty() {
        conn.execute(
            &format!("DELETE FROM projects WHERE id IN ({})", ids_str),
            [],
        )
        .map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub fn batch_move_category(ids: Vec<i64>, category_id: Option<i64>) -> Result<(), String> {
    let db = DATABASE.lock().map_err(|e| e.to_string())?;
    let db = db.as_ref().ok_or("Database not initialized")?;
    let conn = db.get_connection();
    let ids_str = ids.iter().map(|i| i.to_string()).collect::<Vec<_>>().join(",");
    if !ids_str.is_empty() {
        let category_value = match category_id {
            Some(id) => id.to_string(),
            None => "NULL".to_string(),
        };
        conn.execute(
            &format!("UPDATE projects SET category_id = {}, updated_at = datetime('now', 'localtime') WHERE id IN ({})", category_value, ids_str),
            [],
        )
        .map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub fn batch_restore_projects(ids: Vec<i64>) -> Result<(), String> {
    let db = DATABASE.lock().map_err(|e| e.to_string())?;
    let db = db.as_ref().ok_or("Database not initialized")?;
    let conn = db.get_connection();
    let ids_str = ids.iter().map(|i| i.to_string()).collect::<Vec<_>>().join(",");
    if !ids_str.is_empty() {
        conn.execute(
            &format!("UPDATE projects SET data_status = 'ACTIVE', archived_at = NULL, updated_at = datetime('now', 'localtime') WHERE id IN ({})", ids_str),
            [],
        )
        .map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub fn get_category(id: i64) -> Result<Option<Category>, String> {
    let db = DATABASE.lock().map_err(|e| e.to_string())?;
    let db = db.as_ref().ok_or("Database not initialized")?;
    let conn = db.get_connection();
    let mut stmt = conn
        .prepare(
            "SELECT c.id, c.name, c.parent_id, c.sort_order, c.created_at, c.updated_at, \
             (SELECT COUNT(*) FROM projects p WHERE p.category_id = c.id AND p.data_status = 'ACTIVE') \
             FROM categories c WHERE c.id = ?",
        )
        .map_err(|e| e.to_string())?;
    let category = stmt
        .query_row([id], |row| {
            let cid: i64 = row.get(0)?;
            Ok(Category {
                id: cid,
                name: row.get(1)?,
                path: format!("/{}", row.get::<_, String>(1).unwrap_or_default()),
                explain: None,
                sort_order: row.get(3)?,
                parent_id: row.get(2)?,
                is_system: cid == 1,
                project_count: row.get(6)?,
                created_at: row.get(4)?,
                updated_at: row.get(5)?,
            })
        })
        .ok();
    Ok(category)
}

#[tauri::command]
pub fn create_category(input: serde_json::Value) -> Result<i64, String> {
    let db = DATABASE.lock().map_err(|e| e.to_string())?;
    let db = db.as_ref().ok_or("Database not initialized")?;
    let conn = db.get_connection();
    let name: String = serde_json::from_value(input.get("name").unwrap_or(&serde_json::Value::Null).clone()).map_err(|e| e.to_string())?;
    let parent_id: Option<i64> = serde_json::from_value(input.get("parent_id").unwrap_or(&serde_json::Value::Null).clone()).ok();
    let sort_order: i32 = serde_json::from_value(input.get("sort_order").unwrap_or(&serde_json::Value::Null).clone()).unwrap_or(0);

    // 系统分类（未分类 id=1）不能作为父分类
    if parent_id == Some(1) {
        return Err("系统分类“未分类”不可作为父分类".to_string());
    }

    conn.execute(
        "INSERT INTO categories (name, parent_id, sort_order, created_at, updated_at) VALUES (?, ?, ?, datetime('now', 'localtime'), datetime('now', 'localtime'))",
        rusqlite::params![name, parent_id, sort_order],
    )
    .map_err(|e| e.to_string())?;
    Ok(conn.last_insert_rowid())
}

#[tauri::command]
pub fn update_category(input: Category) -> Result<(), String> {
    let db = DATABASE.lock().map_err(|e| e.to_string())?;
    let db = db.as_ref().ok_or("Database not initialized")?;
    let conn = db.get_connection();

    // 禁止将分类移到系统分类（未分类）下
    if input.parent_id == Some(1) {
        return Err("系统分类“未分类”不可作为父分类".to_string());
    }

    // 系统分类（id=1）不能修改 name/parent_id/sort_order
    if input.id == 1 {
        return Err("系统分类“未分类”不可修改".to_string());
    }

    conn.execute(
        "UPDATE categories SET name = ?, parent_id = ?, sort_order = ?, updated_at = datetime('now', 'localtime') WHERE id = ?",
        rusqlite::params![input.name, input.parent_id, input.sort_order, input.id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn delete_category(id: i64) -> Result<(), String> {
    let db = DATABASE.lock().map_err(|e| e.to_string())?;
    let db = db.as_ref().ok_or("Database not initialized")?;
    let conn = db.get_connection();

    // 系统分类不可删除
    if id == 1 {
        return Err("系统分类“未分类”不可删除".to_string());
    }

    // 有子分类不可删除
    let child_count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM categories WHERE parent_id = ?",
        [id],
        |row| row.get(0),
    ).map_err(|e| e.to_string())?;
    if child_count > 0 {
        return Err("该分类下存在子分类，无法删除".to_string());
    }

    // 有项目不可删除
    let project_count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM projects WHERE category_id = ? AND data_status = 'ACTIVE'",
        [id],
        |row| row.get(0),
    ).map_err(|e| e.to_string())?;
    if project_count > 0 {
        return Err(format!("该分类下有 {} 个项目，无法删除", project_count));
    }

    conn.execute("DELETE FROM categories WHERE id = ?", [id])
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn get_tags() -> Result<Vec<Tag>, String> {
    let db = DATABASE.lock().map_err(|e| e.to_string())?;
    let db = db.as_ref().ok_or("Database not initialized")?;
    let conn = db.get_connection();
    let mut stmt = conn
        .prepare("SELECT id, name, color, source, created_at FROM tags ORDER BY name")
        .map_err(|e| e.to_string())?;
    let tags = stmt
        .query_map([], |row| {
            Ok(Tag {
                id: row.get(0)?,
                name: row.get(1)?,
                color: row.get(2)?,
                source: row.get::<_, Option<String>>(3)?.unwrap_or_else(|| "user".to_string()),
                created_at: row.get(4)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;
    Ok(tags)
}

#[tauri::command]
pub fn get_project_tags(project_id: i64) -> Result<Vec<Tag>, String> {
    let db = DATABASE.lock().map_err(|e| e.to_string())?;
    let db = db.as_ref().ok_or("Database not initialized")?;
    let conn = db.get_connection();
    let mut stmt = conn
        .prepare("SELECT t.id, t.name, t.color, t.source, t.created_at FROM tags t INNER JOIN project_tags pt ON t.id = pt.tag_id WHERE pt.project_id = ?")
        .map_err(|e| e.to_string())?;
    let tags = stmt
        .query_map([project_id], |row| {
            Ok(Tag {
                id: row.get(0)?,
                name: row.get(1)?,
                color: row.get(2)?,
                source: row.get::<_, Option<String>>(3)?.unwrap_or_else(|| "user".to_string()),
                created_at: row.get(4)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;
    Ok(tags)
}

#[tauri::command]
pub fn create_tag(input: serde_json::Value) -> Result<i64, String> {
    let db = DATABASE.lock().map_err(|e| e.to_string())?;
    let db = db.as_ref().ok_or("Database not initialized")?;
    let conn = db.get_connection();
    let name: String = serde_json::from_value(input.get("name").unwrap_or(&serde_json::Value::Null).clone()).map_err(|e| e.to_string())?;
    let color: String = serde_json::from_value(input.get("color").unwrap_or(&serde_json::json!("#6366f1")).clone()).unwrap_or_default();
    conn.execute(
        "INSERT INTO tags (name, color, created_at) VALUES (?, ?, datetime('now', 'localtime'))",
        rusqlite::params![name, color],
    )
    .map_err(|e| e.to_string())?;
    Ok(conn.last_insert_rowid())
}

#[tauri::command]
pub fn add_tag_to_project(project_id: i64, tag_id: i64) -> Result<(), String> {
    let db = DATABASE.lock().map_err(|e| e.to_string())?;
    let db = db.as_ref().ok_or("Database not initialized")?;
    let conn = db.get_connection();
    conn.execute(
        "INSERT OR IGNORE INTO project_tags (project_id, tag_id) VALUES (?, ?)",
        rusqlite::params![project_id, tag_id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn remove_tag_from_project(project_id: i64, tag_id: i64) -> Result<(), String> {
    let db = DATABASE.lock().map_err(|e| e.to_string())?;
    let db = db.as_ref().ok_or("Database not initialized")?;
    let conn = db.get_connection();
    conn.execute(
        "DELETE FROM project_tags WHERE project_id = ? AND tag_id = ?",
        rusqlite::params![project_id, tag_id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn delete_tag(id: i64) -> Result<(), String> {
    let db = DATABASE.lock().map_err(|e| e.to_string())?;
    let db = db.as_ref().ok_or("Database not initialized")?;
    let conn = db.get_connection();
    conn.execute("DELETE FROM tags WHERE id = ?", [id])
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn get_project_notes(project_id: i64) -> Result<Vec<ProjectNote>, String> {
    let db = DATABASE.lock().map_err(|e| e.to_string())?;
    let db = db.as_ref().ok_or("Database not initialized")?;
    let conn = db.get_connection();
    let mut stmt = conn
        .prepare("SELECT id, project_id, content, created_at, updated_at FROM project_notes WHERE project_id = ? ORDER BY updated_at DESC")
        .map_err(|e| e.to_string())?;
    let notes = stmt
        .query_map([project_id], |row| {
            Ok(ProjectNote {
                id: row.get(0)?,
                project_id: row.get(1)?,
                content: row.get(2)?,
                created_at: row.get(3)?,
                updated_at: row.get(4)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;
    Ok(notes)
}

#[tauri::command]
pub fn create_note(input: serde_json::Value) -> Result<i64, String> {
    let db = DATABASE.lock().map_err(|e| e.to_string())?;
    let db = db.as_ref().ok_or("Database not initialized")?;
    let conn = db.get_connection();
    let project_id: i64 = serde_json::from_value(input.get("project_id").unwrap_or(&serde_json::Value::Null).clone()).map_err(|e| e.to_string())?;
    let content: String = serde_json::from_value(input.get("content").unwrap_or(&serde_json::Value::Null).clone()).map_err(|e| e.to_string())?;
    conn.execute(
        "INSERT INTO project_notes (project_id, content, created_at, updated_at) VALUES (?, ?, datetime('now', 'localtime'), datetime('now', 'localtime'))",
        rusqlite::params![project_id, content],
    )
    .map_err(|e| e.to_string())?;
    Ok(conn.last_insert_rowid())
}

#[tauri::command]
pub fn update_note(input: serde_json::Value) -> Result<(), String> {
    let db = DATABASE.lock().map_err(|e| e.to_string())?;
    let db = db.as_ref().ok_or("Database not initialized")?;
    let conn = db.get_connection();
    let id: i64 = serde_json::from_value(input.get("id").unwrap_or(&serde_json::Value::Null).clone()).map_err(|e| e.to_string())?;
    let content: String = serde_json::from_value(input.get("content").unwrap_or(&serde_json::Value::Null).clone()).map_err(|e| e.to_string())?;
    conn.execute(
        "UPDATE project_notes SET content = ?, updated_at = datetime('now', 'localtime') WHERE id = ?",
        rusqlite::params![content, id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn delete_note(id: i64) -> Result<(), String> {
    let db = DATABASE.lock().map_err(|e| e.to_string())?;
    let db = db.as_ref().ok_or("Database not initialized")?;
    let conn = db.get_connection();
    conn.execute("DELETE FROM project_notes WHERE id = ?", [id])
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn archive_project(id: i64) -> Result<(), String> {
    let db = DATABASE.lock().map_err(|e| e.to_string())?;
    let db = db.as_ref().ok_or("Database not initialized")?;
    let conn = db.get_connection();
    conn.execute(
        "UPDATE projects SET data_status = 'ARCHIVED', archived_at = datetime('now', 'localtime'), updated_at = datetime('now', 'localtime') WHERE id = ?",
        [id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn restore_project(id: i64) -> Result<(), String> {
    let db = DATABASE.lock().map_err(|e| e.to_string())?;
    let db = db.as_ref().ok_or("Database not initialized")?;
    let conn = db.get_connection();
    conn.execute(
        "UPDATE projects SET data_status = 'ACTIVE', archived_at = NULL, updated_at = datetime('now', 'localtime') WHERE id = ?",
        [id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn get_archived_projects() -> Result<Vec<Project>, String> {
    let db = DATABASE.lock().map_err(|e| e.to_string())?;
    let db = db.as_ref().ok_or("Database not initialized")?;
    let conn = db.get_connection();
    let mut stmt = conn
        .prepare(
            "SELECT id, name, url, source, source_id, description, languages, stars, forks, open_issues, license, homepage, latest_commit, latest_release, readme_content, readme_lang, health_score, ai_summary, ai_use_cases, ai_risks, ai_dependencies, translated_summary, translated_use_cases, translated_risks, translated_dependencies, translated_description, readme_translation, category_id, lifecycle_status, data_status, is_downloaded, local_path, runbook, archived_at, deleted_at, created_at, updated_at FROM projects WHERE data_status = 'ARCHIVED' ORDER BY archived_at DESC"
        )
        .map_err(|e| e.to_string())?;
    let projects = stmt
        .query_map([], |row| {
            Ok(Project {
                id: row.get(0)?,
                name: row.get(1)?,
                url: row.get(2)?,
                source: row.get(3)?,
                source_id: row.get(4)?,
                description: row.get(5)?,
                languages: row.get(6)?,
                stars: row.get(7)?,
                forks: row.get(8)?,
                open_issues: row.get(9)?,
                license: row.get(10)?,
                homepage: row.get(11)?,
                latest_commit: row.get(12)?,
                latest_release: row.get(13)?,
                readme_content: row.get(14)?,
                readme_lang: row.get(15)?,
                health_score: row.get(16)?,
                ai_summary: row.get(17)?,
                ai_use_cases: row.get(18)?,
                ai_risks: row.get(19)?,
                ai_dependencies: row.get(20)?,
                translated_summary: row.get(21)?,
                translated_use_cases: row.get(22)?,
                translated_risks: row.get(23)?,
                translated_dependencies: row.get(24)?,
                translated_description: row.get(25)?,
                readme_translation: row.get(26)?,
                category_id: row.get(27)?,
                lifecycle_status: row.get(28)?,
                data_status: row.get(29)?,
                is_downloaded: row.get::<_, i32>(30)? != 0,
                local_path: row.get(31)?,
                runbook: row.get(32)?,
                archived_at: row.get(33)?,
                deleted_at: row.get(34)?,
                created_at: row.get(35)?,
                updated_at: row.get(36)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;
    Ok(projects)
}
