use crate::db::DATABASE;
use chrono::Local;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectClone {
    pub id: i64,
    pub project_id: i64,
    pub cloned_path: Option<String>,
    pub cloned_at: Option<String>,
    pub proxy_enabled: bool,
    pub proxy_protocol: String,
    pub proxy_host: String,
    pub proxy_port: i32,
    pub proxy_username: String,
    pub proxy_password: Option<String>,
}

fn row_to_project_clone(row: &rusqlite::Row) -> rusqlite::Result<ProjectClone> {
    Ok(ProjectClone {
        id: row.get(0)?,
        project_id: row.get(1)?,
        cloned_path: row.get(2)?,
        cloned_at: row.get(3)?,
        proxy_enabled: row.get::<_, i32>(4)? != 0,
        proxy_protocol: row.get::<_, Option<String>>(5)?.unwrap_or_else(|| "http".to_string()),
        proxy_host: row.get::<_, Option<String>>(6)?.unwrap_or_default(),
        proxy_port: row.get::<_, Option<i32>>(7)?.unwrap_or(0),
        proxy_username: row.get::<_, Option<String>>(8)?.unwrap_or_default(),
        proxy_password: row.get(9)?,
    })
}

#[tauri::command]
pub fn path_exists(path: String) -> bool {
    std::path::Path::new(&path).exists()
}

#[tauri::command]
pub fn get_clone_settings(project_id: i64) -> Result<ProjectClone, String> {
    let db_guard = DATABASE.try_lock().map_err(|e| e.to_string())?;
    let db = db_guard.as_ref().ok_or("Database not initialized")?;
    let conn = db.get_connection();

    let result = conn.query_row(
        "SELECT id, project_id, cloned_path, cloned_at, proxy_enabled, proxy_protocol, proxy_host, proxy_port, proxy_username, proxy_password FROM project_clone WHERE project_id = ?",
        [project_id],
        row_to_project_clone,
    );

    match result {
        Ok(data) => Ok(data),
        Err(_) => {
            conn.execute(
                "INSERT INTO project_clone (project_id) VALUES (?)",
                [project_id],
            ).map_err(|e| format!("INSERT failed: {}", e))?;

            conn.query_row(
                "SELECT id, project_id, cloned_path, cloned_at, proxy_enabled, proxy_protocol, proxy_host, proxy_port, proxy_username, proxy_password FROM project_clone WHERE project_id = ?",
                [project_id],
                row_to_project_clone,
            ).map_err(|e| format!("Query after insert failed: {}", e))
        }
    }
}

#[tauri::command]
pub fn save_clone_settings(clone: ProjectClone) -> Result<(), String> {
    let db_guard = DATABASE.try_lock().map_err(|e| e.to_string())?;
    let db = db_guard.as_ref().ok_or("Database not initialized")?;
    let conn = db.get_connection();

    conn.execute(
        "UPDATE project_clone SET cloned_path = ?, cloned_at = ?, proxy_enabled = ?, proxy_protocol = ?, proxy_host = ?, proxy_port = ?, proxy_username = ?, proxy_password = ? WHERE project_id = ?",
        rusqlite::params![
            clone.cloned_path,
            clone.cloned_at,
            if clone.proxy_enabled { 1 } else { 0 },
            clone.proxy_protocol,
            clone.proxy_host,
            clone.proxy_port,
            clone.proxy_username,
            clone.proxy_password,
            clone.project_id,
        ],
    ).map_err(|e| format!("UPDATE failed: {}", e))?;

    Ok(())
}

#[tauri::command]
pub fn execute_clone(project_id: i64, git_url: String, project_name: String) -> Result<String, String> {
    let settings = get_clone_settings(project_id)?;

    let cloned_path = settings.cloned_path.ok_or("请先设置并保存克隆目录")?;
    let full_path = format!("{}/{}", cloned_path, project_name);

    if std::path::Path::new(&full_path).exists() {
        return Err(format!("目录已存在: {}", full_path));
    }

    let output = std::process::Command::new("git")
        .args(["clone", &git_url, &full_path])
        .output()
        .map_err(|e| format!("Git failed: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Clone failed: {}", stderr));
    }

    let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();

    let updated = ProjectClone {
        id: settings.id,
        project_id: settings.project_id,
        cloned_path: Some(full_path.clone()),
        cloned_at: Some(timestamp),
        proxy_enabled: settings.proxy_enabled,
        proxy_protocol: settings.proxy_protocol,
        proxy_host: settings.proxy_host,
        proxy_port: settings.proxy_port,
        proxy_username: settings.proxy_username,
        proxy_password: settings.proxy_password,
    };

    save_clone_settings(updated)?;
    Ok(full_path)
}
