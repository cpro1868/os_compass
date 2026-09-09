use crate::db::DATABASE;
use std::collections::HashMap;
use crate::models::{ReadmeVariant, Translation};
use crate::plugins;
use crate::plugins::{npm, pypi, crates};
use log;

#[tauri::command]
pub async fn refresh_project_readme(project_id: i64) -> Result<RefreshReadmeResult, String> {
    let (url, source) = {
        let db = DATABASE.lock().map_err(|e| e.to_string())?;
        let db = db.as_ref().ok_or("Database not initialized")?;
        let conn = db.get_connection();
        let row: (String, String) = conn.query_row(
            "SELECT url, source FROM projects WHERE id = ?",
            [project_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        ).map_err(|e| e.to_string())?;
        row
    };

    let platform = plugins::identify_platform(&url);
    let result = match platform {
        plugins::Platform::GitHub => {
            let token = crate::commands::variables::get_variable_value_internal("github_token");
            let proxy = crate::commands::variables::get_variable_value_internal("github_proxy");
            crate::plugins::github::fetch_github_project(&url, token.as_deref(), proxy.as_deref()).await
        }
        plugins::Platform::Gitee => {
            let token = crate::commands::variables::get_variable_value_internal("gitee_token");
            crate::plugins::gitee::fetch_gitee_project(&url, token.as_deref()).await
        }
        plugins::Platform::NPM => {
            npm::fetch_npm_project(&url, None).await
        }
        plugins::Platform::PyPI => {
            pypi::fetch_pypi_project(&url, None).await
        }
        plugins::Platform::Crates => {
            crates::fetch_crates_project(&url, None).await
        }
        plugins::Platform::Unknown => {
            return Err("不支持的平台，无法刷新 README".to_string());
        }
    };

    if !result.success {
        return Err(result.error.unwrap_or_else(|| "刷新 README 失败".to_string()));
    }

    let data = result.project_data.ok_or("未获取到项目数据")?;
    let readme_content = data.readme_content;
    let variants = result.readme_variants;

    {
        let db = DATABASE.lock().map_err(|e| e.to_string())?;
        let db = db.as_ref().ok_or("Database not initialized")?;
        let conn = db.get_connection();

        if let Some(ref content) = readme_content {
            conn.execute(
                "UPDATE projects SET readme_content = ?, updated_at = datetime('now', 'localtime') WHERE id = ?",
                rusqlite::params![content, project_id],
            ).map_err(|e| e.to_string())?;
        }

        if let Some(ref variants) = variants {
            conn.execute("DELETE FROM readme_variants WHERE project_id = ?", [project_id]).map_err(|e| e.to_string())?;
            for (file_name, content) in variants {
                let language = detect_readme_lang(&file_name);
                conn.execute(
                    "INSERT INTO readme_variants (project_id, file_name, language, content) VALUES (?, ?, ?, ?)",
                    rusqlite::params![project_id, file_name, language, content],
                ).map_err(|e| e.to_string())?;
            }
        }
    }

    Ok(RefreshReadmeResult {
        readme_content,
        variants_count: variants.map(|v| v.len()).unwrap_or(0),
    })
}

#[derive(Debug, serde::Serialize)]
pub struct RefreshReadmeResult {
    pub readme_content: Option<String>,
    pub variants_count: usize,
}

#[tauri::command]
pub fn get_readme_variants_cmd(project_id: i64) -> Result<Vec<ReadmeVariant>, String> {
    let db = DATABASE.lock().map_err(|e| e.to_string())?;
    let db = db.as_ref().ok_or("Database not initialized")?;
    let conn = db.get_connection();
    let mut stmt = conn.prepare(
        "SELECT id, project_id, file_name, language, content, created_at FROM readme_variants WHERE project_id = ? ORDER BY language"
    ).map_err(|e| e.to_string())?;
    let variants = stmt.query_map([project_id], |row| {
        Ok(ReadmeVariant {
            id: row.get(0)?,
            project_id: row.get(1)?,
            file_name: row.get(2)?,
            language: row.get(3)?,
            content: row.get(4)?,
            created_at: row.get(5)?,
        })
    }).map_err(|e| e.to_string())?
    .collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())?;
    Ok(variants)
}

#[tauri::command]
pub fn save_readme_variants_cmd(project_id: i64, variants: HashMap<String, String>) -> Result<(), String> {
    let db = DATABASE.lock().map_err(|e| e.to_string())?;
    let db = db.as_ref().ok_or("Database not initialized")?;
    let conn = db.get_connection();

    conn.execute("DELETE FROM readme_variants WHERE project_id = ?", [project_id]).map_err(|e| e.to_string())?;

    for (file_name, content) in variants {
        let language = detect_readme_lang(&file_name);
        conn.execute(
            "INSERT INTO readme_variants (project_id, file_name, language, content) VALUES (?, ?, ?, ?)",
            rusqlite::params![project_id, file_name, language, content],
        ).map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub fn get_translations_cmd(project_id: i64, language: String) -> Result<Vec<Translation>, String> {
    log::debug!("[GET_TRANSLATIONS] project_id={}, language={}", project_id, language);
    let db = DATABASE.lock().map_err(|e| e.to_string())?;
    let db = db.as_ref().ok_or("Database not initialized")?;
    let conn = db.get_connection();

    conn.execute(
        "CREATE TABLE IF NOT EXISTS translations (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            project_id INTEGER NOT NULL,
            field_name TEXT NOT NULL,
            language TEXT NOT NULL,
            content TEXT NOT NULL,
            created_at TEXT DEFAULT (datetime('now', 'localtime')),
            updated_at TEXT DEFAULT (datetime('now', 'localtime')),
            UNIQUE(project_id, field_name, language)
        )",
        [],
    ).ok();

    let count: i64 = conn.query_row("SELECT COUNT(*) FROM translations WHERE project_id = ? AND language = ?", rusqlite::params![project_id, language], |row| row.get(0)).unwrap_or(0);
    log::debug!("[GET_TRANSLATIONS] found {} records in DB", count);

    let mut stmt = conn.prepare(
        "SELECT id, project_id, field_name, language, content, created_at, updated_at FROM translations WHERE project_id = ? AND language = ?"
    ).map_err(|e| e.to_string())?;
    let translations = stmt.query_map(rusqlite::params![project_id, language], |row| {
        Ok(Translation {
            id: row.get(0)?,
            project_id: row.get(1)?,
            field_name: row.get(2)?,
            language: row.get(3)?,
            content: row.get(4)?,
            created_at: row.get(5)?,
            updated_at: row.get(6)?,
        })
    }).map_err(|e| e.to_string())?
    .collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())?;
    log::debug!("[GET_TRANSLATIONS] returning {} records", translations.len());
    Ok(translations)
}

#[tauri::command]
pub fn save_translation_cmd(project_id: i64, field_name: String, language: String, content: String) -> Result<(), String> {
    let db = DATABASE.lock().map_err(|e| e.to_string())?;
    let db = db.as_ref().ok_or("Database not initialized")?;
    let conn = db.get_connection();

    conn.execute(
        "INSERT OR REPLACE INTO translations (project_id, field_name, language, content, updated_at) VALUES (?, ?, ?, ?, datetime('now', 'localtime'))",
        rusqlite::params![project_id, field_name, language, content],
    ).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn delete_translation_cmd(project_id: i64, field_name: String, language: String) -> Result<(), String> {
    let db = DATABASE.lock().map_err(|e| e.to_string())?;
    let db = db.as_ref().ok_or("Database not initialized")?;
    let conn = db.get_connection();

    conn.execute(
        "DELETE FROM translations WHERE project_id = ? AND field_name = ? AND language = ?",
        rusqlite::params![project_id, field_name, language],
    ).map_err(|e| e.to_string())?;
    Ok(())
}

pub fn detect_readme_lang(file_name: &str) -> String {
    let lower = file_name.to_lowercase();
    if lower.contains("_zh") || lower.contains(".zh") || lower.contains("_cn") || lower.contains(".cn") || lower.contains("-zh") {
        "zh-CN".to_string()
    } else if lower.contains("_hant") || lower.contains(".hant") || lower.contains("_tw") || lower.contains("-tw") {
        "zh-TW".to_string()
    } else if lower.contains("_ja") || lower.contains(".ja") || lower.contains("-ja") {
        "ja-JP".to_string()
    } else if lower.contains("_ko") || lower.contains(".ko") || lower.contains("-ko") {
        "ko-KR".to_string()
    } else if lower.contains("_en") || lower.contains(".en") || lower == "readme.md" {
        "en".to_string()
    } else if lower.contains("_es") || lower.contains(".es") {
        "es".to_string()
    } else if lower.contains("_fr") || lower.contains(".fr") {
        "fr".to_string()
    } else if lower.contains("_de") || lower.contains(".de") {
        "de".to_string()
    } else if lower.contains("_ru") || lower.contains(".ru") {
        "ru".to_string()
    } else {
        "unknown".to_string()
    }
}

#[tauri::command]
pub fn debug_all_translations() -> Result<String, String> {
    let db = DATABASE.lock().map_err(|e| e.to_string())?;
    let db = db.as_ref().ok_or("Database not initialized")?;
    let conn = db.get_connection();

    conn.execute(
        "CREATE TABLE IF NOT EXISTS translations (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            project_id INTEGER NOT NULL,
            field_name TEXT NOT NULL,
            language TEXT NOT NULL,
            content TEXT NOT NULL,
            created_at TEXT DEFAULT (datetime('now', 'localtime')),
            updated_at TEXT DEFAULT (datetime('now', 'localtime')),
            UNIQUE(project_id, field_name, language)
        )",
        [],
    ).ok();

    let mut stmt = conn.prepare(
        "SELECT project_id, field_name, language, length(content) as content_len FROM translations ORDER BY project_id, field_name"
    ).map_err(|e| e.to_string())?;

    let mut result = String::from("Translations table:\n");
    let rows = stmt.query_map([], |row| {
        Ok(format!("  project_id={}, field={}, lang={}, len={}",
            row.get::<_, i64>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, i32>(3)?
        ))
    }).map_err(|e| e.to_string())?;

    for row in rows {
        match row {
            Ok(r) => {
                result.push_str(&r);
                result.push('\n');
            }
            Err(e) => {
                result.push_str(&format!("Error: {}\n", e));
            }
        }
    }

    let count: i64 = conn.query_row("SELECT COUNT(*) FROM translations", [], |row| row.get(0)).unwrap_or(0);
    result.push_str(&format!("Total: {} records\n", count));

    Ok(result)
}
