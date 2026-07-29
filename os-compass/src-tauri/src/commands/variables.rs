use crate::db::DATABASE;
use crate::crypto;
use log;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SystemVariable {
    pub key: String,
    pub value: Option<String>,
    #[serde(rename = "isSecret")]
    pub is_secret: bool,
    #[serde(rename = "createdAt")]
    pub created_at: Option<String>,
    #[serde(rename = "updatedAt")]
    pub updated_at: Option<String>,
}

impl SystemVariable {
    pub fn to_js(&self) -> serde_json::Value {
        serde_json::json!({
            "key": self.key,
            "value": self.value,
            "isSecret": self.is_secret,
            "createdAt": self.created_at,
            "updatedAt": self.updated_at,
        })
    }
}

#[tauri::command]
pub fn get_system_variables() -> Result<Vec<serde_json::Value>, String> {
    let db = DATABASE.lock().map_err(|e| e.to_string())?;
    let db = db.as_ref().ok_or("Database not initialized")?;
    let conn = db.get_connection();
    let mut stmt = conn
        .prepare("SELECT key, value, is_secret, created_at, updated_at FROM system_variables ORDER BY key")
        .map_err(|e| e.to_string())?;
    let variables = stmt
        .query_map([], |row| {
            let value: Option<String> = row.get(1)?;
            let is_secret: bool = row.get::<_, i32>(2)? != 0;
            let decrypted_value = if is_secret {
                match value.as_ref() {
                    Some(v) if !v.is_empty() => {
                        match crypto::decrypt_string(v) {
                            Ok(plain) => Some(plain),
                            Err(e) => {
                                let key: String = row.get::<_, String>(0)?;
                                log::warn!("[get_system_variables] WARNING: decrypt failed for key='{}', stored_len={}, err={}. Returning empty.", key, v.len(), e);
                                Some(String::new())
                            }
                        }
                    }
                    _ => value.clone(),
                }
            } else {
                value
            };
            Ok(serde_json::json!({
                "key": row.get::<_, String>(0)?,
                "value": decrypted_value,
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
    let db = DATABASE.lock().map_err(|e| e.to_string())?;
    let db = db.as_ref().ok_or("Database not initialized")?;
    let conn = db.get_connection();
    let mut stmt = conn
        .prepare("SELECT key, value, is_secret, created_at, updated_at FROM system_variables WHERE key = ?")
        .map_err(|e| e.to_string())?;
    let result = stmt
        .query_row([&key], |row| {
            let value: Option<String> = row.get(1)?;
            let is_secret: bool = row.get::<_, i32>(2)? != 0;
            let decrypted_value = if is_secret {
                value.and_then(|v| crypto::decrypt_string(&v).ok())
            } else {
                value
            };
            Ok(serde_json::json!({
                "key": row.get::<_, String>(0)?,
                "value": decrypted_value,
                "isSecret": is_secret,
                "createdAt": row.get::<_, Option<String>>(3)?,
                "updatedAt": row.get::<_, Option<String>>(4)?,
            }))
        })
        .ok();
    Ok(result)
}
#[tauri::command]
pub fn set_system_variable(key: String, value: String, is_secret: bool) -> Result<(), String> {
    let db = DATABASE.lock().map_err(|e| e.to_string())?;
    let db = db.as_ref().ok_or("Database not initialized")?;
    let conn = db.get_connection();

    let stored_value = if is_secret {
        crypto::encrypt_string(&value)?
    } else {
        value.clone()
    };

    conn.execute(
        "INSERT INTO system_variables (key, value, is_secret, updated_at) VALUES (?, ?, ?, datetime('now', 'localtime'))
         ON CONFLICT(key) DO UPDATE SET value = excluded.value, is_secret = excluded.is_secret, updated_at = datetime('now', 'localtime')",
        rusqlite::params![key, stored_value, is_secret as i32],
    )
    .map_err(|e| e.to_string())?;

    log::debug!("[variables] Saved: key={}, is_secret={}, stored_len={}", key, is_secret, stored_value.len());

    Ok(())
}

#[tauri::command]
pub fn delete_system_variable(key: String) -> Result<(), String> {
    let db = DATABASE.lock().map_err(|e| e.to_string())?;
    let db = db.as_ref().ok_or("Database not initialized")?;
    let conn = db.get_connection();
    conn.execute("DELETE FROM system_variables WHERE key = ?", rusqlite::params![key])
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn get_variable(key: String) -> Result<Option<String>, String> {
    Ok(get_variable_value_internal(&key))
}

pub fn get_variable_value_internal(key: &str) -> Option<String> {
    let db = match DATABASE.lock() {
        Ok(db) => db,
        Err(_) => return None,
    };
    let db = db.as_ref()?;
    let conn = db.get_connection();
    let mut stmt = match conn.prepare("SELECT value, is_secret FROM system_variables WHERE key = ?") {
        Ok(stmt) => stmt,
        Err(_) => return None,
    };
    let result: (Option<String>, i32) = stmt.query_row([key], |row| {
        Ok((row.get(0)?, row.get(1)?))
    }).ok()?;
    let (value, is_secret) = result;
    match value {
        Some(v) if is_secret != 0 => {
            if v.is_empty() {
                return None;
            }
            match crypto::decrypt_string(&v) {
                Ok(plain) => Some(plain),
                Err(e) => {
                    // 解密失败：保留密文，不清理，避免密钥临时不匹配导致数据永久丢失
                    log::warn!("[variables] WARNING: Cannot decrypt secret '{}' (len={}): {}. Data preserved.", key, v.len(), e);
                    None
                }
            }
        }
        Some(v) => Some(v),
        None => None,
    }
}

/// 启动时检查 system_variables 中无法用当前密钥解密的旧密文
/// 仅记录警告日志，不再清空数据，避免密钥临时不匹配导致数据丢失
pub fn cleanup_undecryptable_secrets() {
    let db = match DATABASE.lock() {
        Ok(db) => db,
        Err(_) => return,
    };
    let db = match db.as_ref() {
        Some(db) => db,
        None => return,
    };
    let conn = db.get_connection();

    // 查询所有敏感变量
    let keys: Vec<(String, String)> = match conn.prepare(
        "SELECT key, value FROM system_variables WHERE is_secret = 1 AND value != ''"
    ) {
        Ok(mut stmt) => {
            match stmt.query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            }) {
                Ok(rows) => rows.filter_map(|r| r.ok()).collect(),
                Err(_) => return,
            }
        }
        Err(_) => return,
    };

    let mut undecryptable = 0;
    for (key, value) in keys {
        if crypto::decrypt_string(&value).is_err() {
            undecryptable += 1;
            log::warn!("[variables] WARNING: Cannot decrypt secret '{}' with current key. Data preserved (not cleared).", key);
        }
    }

    if undecryptable > 0 {
        log::warn!("[variables] WARNING: {} secret(s) could not be decrypted with current crypto key. Check .cryptokey file matches the vault.", undecryptable);
    }
}
