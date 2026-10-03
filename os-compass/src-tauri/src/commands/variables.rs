//! 系统变量命令层（M24 架构修正：全部读写切换到系统库 system_settings.db）
//! 系统变量（Token/代理等）属于全局配置，恒定不随仓库切换；仓库库中的旧表仅作迁移数据源。

use crate::crypto;
use crate::db::DATABASE;
use crate::system_db;
use log;

#[tauri::command]
pub fn get_system_variables() -> Result<Vec<serde_json::Value>, String> {
    let db_lock = system_db::SYSTEM_DB.lock().map_err(|e| e.to_string())?;
    let db = db_lock.as_ref().ok_or("System DB not initialized")?;
    let conn = db.get_connection();
    let mut stmt = conn
        .prepare("SELECT key, encrypted_value, is_secret, created_at, updated_at FROM system_variables ORDER BY key")
        .map_err(|e| e.to_string())?;
    let variables = stmt
        .query_map([], |row| {
            let raw: String = row.get(1)?;
            let is_secret: bool = row.get::<_, i32>(2)? != 0;
            let value = if is_secret && !raw.is_empty() {
                match crypto::decrypt_string(&raw) {
                    Ok(plain) => Some(plain),
                    Err(e) => {
                        let key: String = row.get::<_, String>(0)?;
                        log::warn!("[get_system_variables] WARNING: decrypt failed for key='{}', stored_len={}, err={}. Returning empty.", key, raw.len(), e);
                        Some(String::new())
                    }
                }
            } else {
                Some(raw)
            };
            Ok(serde_json::json!({
                "key": row.get::<_, String>(0)?,
                "value": value,
                "isSecret": is_secret,
                "createdAt": row.get::<_, Option<String>>(3)?,
                "updatedAt": row.get::<_, Option<String>>(4)?,
            }))
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;
    Ok(variables)
}

#[tauri::command]
pub fn get_system_variable(key: String) -> Result<Option<serde_json::Value>, String> {
    let db_lock = system_db::SYSTEM_DB.lock().map_err(|e| e.to_string())?;
    let db = db_lock.as_ref().ok_or("System DB not initialized")?;
    let conn = db.get_connection();
    let result = conn
        .query_row(
            "SELECT key, encrypted_value, is_secret, created_at, updated_at FROM system_variables WHERE key = ?",
            [&key],
            |row| {
                let raw: String = row.get(1)?;
                let is_secret: bool = row.get::<_, i32>(2)? != 0;
                let value = if is_secret && !raw.is_empty() {
                    crypto::decrypt_string(&raw).ok()
                } else {
                    Some(raw)
                };
                Ok(serde_json::json!({
                    "key": row.get::<_, String>(0)?,
                    "value": value,
                    "isSecret": is_secret,
                    "createdAt": row.get::<_, Option<String>>(3)?,
                    "updatedAt": row.get::<_, Option<String>>(4)?,
                }))
            },
        )
        .ok();
    Ok(result)
}

#[tauri::command]
pub fn set_system_variable(key: String, value: String, is_secret: bool) -> Result<(), String> {
    system_db::set_system_variable(&key, &value, is_secret)?;
    log::debug!("[variables] Saved (system db): key={}, is_secret={}", key, is_secret);
    Ok(())
}

#[tauri::command]
pub fn delete_system_variable(key: String) -> Result<(), String> {
    system_db::delete_system_variable(&key)
}

#[tauri::command]
pub fn get_variable(key: String) -> Result<Option<String>, String> {
    Ok(get_variable_value_internal(&key))
}

pub fn get_variable_value_internal(key: &str) -> Option<String> {
    system_db::get_system_variable(key)
}

/// 将旧版仓库库 system_variables 中的非空变量合并进系统库。
/// 仅当系统库中该 key 不存在或为空时合并，不覆盖系统库已有配置。
/// 仓库库表不存在（新仓库）时静默跳过。
pub fn merge_legacy_vault_variables() {
    // 1. 从仓库库读出全部旧变量（短临界区，不持有仓库锁调系统库函数）
    let legacy: Vec<(String, String, bool)> = {
        let db = match DATABASE.lock() {
            Ok(d) => d,
            Err(_) => return,
        };
        let db = match db.as_ref() {
            Some(d) => d,
            None => return,
        };
        let conn = db.get_connection();
        let mut stmt = match conn.prepare(
            "SELECT key, value, is_secret FROM system_variables WHERE value != ''"
        ) {
            Ok(s) => s,
            Err(_) => return,
        };
        let rows = match stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?, row.get::<_, i32>(2)? != 0))
        }) {
            Ok(r) => r,
            Err(_) => return,
        };
        rows.filter_map(|r| r.ok()).collect()
    };

    for (key, value, is_secret) in legacy {
        let plain = if is_secret {
            match crypto::decrypt_string(&value) {
                Ok(p) => p,
                Err(e) => {
                    log::warn!("[variables] legacy merge: cannot decrypt '{}': {}", key, e);
                    continue;
                }
            }
        } else {
            value
        };
        if plain.is_empty() {
            continue;
        }
        let existing = system_db::get_system_variable(&key);
        if existing.as_deref().map(|s| !s.is_empty()).unwrap_or(false) {
            continue;
        }
        if let Err(e) = system_db::set_system_variable(&key, &plain, is_secret) {
            log::warn!("[variables] legacy merge: failed to set '{}': {}", key, e);
        } else {
            log::info!("[variables] legacy merge: migrated '{}' from vault db to system db", key);
        }
    }
}

/// 启动时检查系统库 system_variables 中无法用当前密钥解密的旧密文
/// 仅记录警告日志，不再清空数据，避免密钥临时不匹配导致数据丢失
pub fn cleanup_undecryptable_secrets() {
    let keys: Vec<(String, String)> = {
        let db_lock = match system_db::SYSTEM_DB.lock() {
            Ok(l) => l,
            Err(_) => return,
        };
        let db = match db_lock.as_ref() {
            Some(d) => d,
            None => return,
        };
        let conn = db.get_connection();
        let mut stmt = match conn.prepare(
            "SELECT key, encrypted_value FROM system_variables WHERE is_secret = 1 AND encrypted_value != ''"
        ) {
            Ok(s) => s,
            Err(_) => return,
        };
        let rows = match stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        }) {
            Ok(r) => r,
            Err(_) => return,
        };
        rows.filter_map(|r| r.ok()).collect()
    };

    let mut undecryptable = 0;
    for (key, value) in keys {
        if crypto::decrypt_string(&value).is_err() {
            undecryptable += 1;
            log::warn!("[variables] WARNING: Cannot decrypt secret '{}' with current key. Data preserved (not cleared).", key);
        }
    }

    if undecryptable > 0 {
        log::warn!("[variables] WARNING: {} secret(s) could not be decrypted with current crypto key. Check .cryptokey file matches.", undecryptable);
    }
}
