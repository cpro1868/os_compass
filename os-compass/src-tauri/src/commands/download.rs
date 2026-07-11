use crate::settings;
use crate::db::DATABASE;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::process::Command;
use tauri::command;

#[derive(Debug, Serialize, Deserialize)]
pub struct CloneResult {
    pub success: bool,
    pub local_path: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OpenResult {
    pub success: bool,
    pub error: Option<String>,
}

fn build_proxy_url() -> Option<String> {
    let s = settings::get_settings();
    if s.proxy_enabled && !s.proxy_host.is_empty() {
        return Some(format!("{}://{}:{}", s.proxy_protocol, s.proxy_host, s.proxy_port));
    }
    None
}

#[command]
pub async fn clone_project(id: i64) -> CloneResult {
    let settings = settings::get_settings();

    if settings.download_path.is_empty() {
        return CloneResult {
            success: false,
            local_path: None,
            error: Some("Download path not configured. Please set it in Settings.".to_string()),
        };
    }

    let (project_url, project_name) = {
        let db = DATABASE.lock().map_err(|e| e.to_string()).unwrap();
        let db = db.as_ref().ok_or("Database not initialized").unwrap();
        let conn = db.get_connection();
        let mut stmt = conn.prepare("SELECT url, name FROM projects WHERE id = ?").unwrap();
        stmt.query_row([id], |row| {
            Ok((row.get::<_, Option<String>>(0)?, row.get::<_, String>(1)?))
        }).unwrap()
    };

    let project_url = match project_url {
        Some(url) => url,
        None => {
            return CloneResult {
                success: false,
                local_path: None,
                error: Some("Project URL not found".to_string()),
            };
        }
    };

    let target_dir = Path::new(&settings.download_path).join(&project_name);

    if target_dir.exists() {
        return CloneResult {
            success: false,
            local_path: None,
            error: Some(format!(
                "Directory already exists: {}",
                target_dir.display()
            )),
        };
    }

    if let Some(parent) = target_dir.parent() {
        if !parent.exists() {
            if let Err(e) = std::fs::create_dir_all(parent) {
                return CloneResult {
                    success: false,
                    local_path: None,
                    error: Some(format!("Failed to create directory: {}", e)),
                };
            }
        }
    }

    let mut cmd = Command::new("git");
    cmd.arg("clone")
       .arg(&project_url)
       .arg(&target_dir)
       .arg("--depth")
       .arg("1");

    if let Some(proxy) = build_proxy_url() {
        cmd.env("HTTPS_PROXY", &proxy);
        cmd.env("HTTP_PROXY", &proxy);
    }

    match cmd.output() {
        Ok(output) => {
            if output.status.success() {
                let local_path = target_dir.display().to_string();
                
                let db = DATABASE.lock().map_err(|e| e.to_string()).unwrap();
                if let Some(db) = db.as_ref() {
                    let conn = db.get_connection();
                    let _ = conn.execute(
                        "UPDATE projects SET is_downloaded = 1, local_path = ? WHERE id = ?",
                        rusqlite::params![&local_path, id],
                    );
                }

                CloneResult {
                    success: true,
                    local_path: Some(local_path),
                    error: None,
                }
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr);
                CloneResult {
                    success: false,
                    local_path: None,
                    error: Some(format!("Git clone failed: {}", stderr)),
                }
            }
        }
        Err(e) => CloneResult {
            success: false,
            local_path: None,
            error: Some(format!("Failed to execute git: {}. Make sure git is installed.", e)),
        },
    }
}

#[command]
pub async fn open_in_editor(path: String, editor: Option<String>) -> OpenResult {
    let settings = settings::get_settings();

    let editor_cmd = editor.unwrap_or(settings.default_editor);

    let mut cmd = Command::new(&editor_cmd);
    cmd.arg(&path);

    if editor_cmd == "code" {
        cmd.arg("--goto");
    }

    match cmd.spawn() {
        Ok(_) => OpenResult {
            success: true,
            error: None,
        },
        Err(e) => OpenResult {
            success: false,
            error: Some(format!(
                "Failed to open {}: {}. Make sure {} is installed and in PATH.",
                editor_cmd, e, editor_cmd
            )),
        },
    }
}
