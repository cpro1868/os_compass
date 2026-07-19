use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tauri::{AppHandle, Manager};
use std::sync::Mutex;
use crate::plugins::radar::init_radar_db;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Vault {
    pub name: String,
    pub path: String,
    pub project_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultInfo {
    pub name: String,
    pub path: String,
}

#[derive(Debug, Serialize)]
pub struct VaultValidation {
    pub valid: bool,
    pub error: Option<String>,
    #[serde(rename = "table_count")]
    pub table_count: Option<i64>,
    #[serde(rename = "project_count")]
    pub project_count: Option<i64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct VaultIndex {
    pub vaults: Vec<VaultInfo>,
}

impl Default for VaultIndex {
    fn default() -> Self {
        Self {
            vaults: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultConfig {
    pub path: String,
}

impl Default for VaultConfig {
    fn default() -> Self {
        Self {
            path: String::new(),
        }
    }
}

lazy_static::lazy_static! {
    pub static ref CURRENT_VAULT_CONFIG: Mutex<Option<VaultConfig>> = Mutex::new(None);
}

fn get_vault_index_path(app: &AppHandle) -> PathBuf {
    let data_dir = app.path().app_data_dir().unwrap_or_default();
    data_dir.join("vault-index.json")
}

fn get_last_vault_path(app: &AppHandle) -> PathBuf {
    let data_dir = app.path().app_data_dir().unwrap_or_default();
    data_dir.join("last-vault.txt")
}

fn get_vaults_root(app: &AppHandle) -> PathBuf {
    let data_dir = app.path().app_data_dir().unwrap_or_default();
    data_dir.join("vaults")
}

pub fn load_vault_index(app: &AppHandle) -> VaultIndex {
    let path = get_vault_index_path(app);
    if path.exists() {
        if let Ok(content) = std::fs::read_to_string(&path) {
            if let Ok(index) = serde_json::from_str(&content) {
                return index;
            }
        }
    }
    VaultIndex::default()
}

pub fn save_vault_index(app: &AppHandle, index: &VaultIndex) -> Result<(), String> {
    let path = get_vault_index_path(app);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let content = serde_json::to_string_pretty(index).map_err(|e| e.to_string())?;
    std::fs::write(&path, content).map_err(|e| e.to_string())?;
    Ok(())
}

pub fn load_vault_config(vault_path: &str) -> VaultConfig {
    let config_path = PathBuf::from(vault_path).join("config.json");
    if config_path.exists() {
        if let Ok(content) = std::fs::read_to_string(&config_path) {
            if let Ok(config) = serde_json::from_str(&content) {
                return config;
            }
        }
    }
    VaultConfig::default()
}

pub fn save_vault_config(vault_path: &str, config: &VaultConfig) -> Result<(), String> {
    let config_path = PathBuf::from(vault_path).join("config.json");
    let content = serde_json::to_string_pretty(config).map_err(|e| e.to_string())?;
    std::fs::write(&config_path, content).map_err(|e| e.to_string())?;
    Ok(())
}

pub fn create_vault(app: AppHandle, name: String, base_path: String) -> Result<Vault, String> {
    // 记录当前仓库路径，创建完成后切回
    let current_db_path = {
        let db_guard = crate::db::DATABASE.lock().unwrap();
        if let Some(ref _db) = *db_guard {
            // 获取当前数据库路径（通过 last-vault.txt）
            let last_vault = std::fs::read_to_string(
                app.path().app_data_dir().unwrap_or_default().join("last-vault.txt")
            ).unwrap_or_default();
            let last_vault = last_vault.trim().to_string();
            if !last_vault.is_empty() {
                Some(std::path::PathBuf::from(last_vault).join("os_compass.db"))
            } else {
                None
            }
        } else {
            None
        }
    };
    let current_crypto_key = {
        let last_vault = std::fs::read_to_string(
            app.path().app_data_dir().unwrap_or_default().join("last-vault.txt")
        ).unwrap_or_default();
        let last_vault = last_vault.trim().to_string();
        if !last_vault.is_empty() {
            let key_path = std::path::PathBuf::from(&last_vault).join(".cryptokey");
            if key_path.exists() {
                std::fs::read_to_string(&key_path)
                    .ok()
                    .and_then(|c| crate::crypto::key_from_base64(c.trim()).ok())
            } else {
                None
            }
        } else {
            None
        }
    };

    // 用户选择的目录就是仓库目录本身，不再在其下创建子目录
    let vault_dir = if !base_path.trim().is_empty() {
        PathBuf::from(base_path.trim())
    } else {
        // 没有选择目录时，使用默认 vaults 目录 + 仓库名
        let vaults_root = get_vaults_root(&app);
        std::fs::create_dir_all(&vaults_root).map_err(|e| e.to_string())?;
        vaults_root.join(&name)
    };

    // 检查是否已经是仓库（已有 os_compass.db）
    let db_path = vault_dir.join("os_compass.db");
    if db_path.exists() {
        return Err("该目录已是仓库目录（os_compass.db 已存在），请选择其他目录".to_string());
    }

    // 目录不存在则创建，已存在则直接使用
    if !vault_dir.exists() {
        std::fs::create_dir_all(&vault_dir).map_err(|e| e.to_string())?;
    }
    let conn = Connection::open(&db_path).map_err(|e| e.to_string())?;
    conn.execute_batch(
        "PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;"
    ).map_err(|e| e.to_string())?;
    drop(conn);

    // 为新仓库初始化数据库表结构（切换到新数据库，初始化后再切回）
    crate::db::switch_database(db_path.clone()).map_err(|e| e.to_string())?;

    // 为新仓库生成独立的加密密钥
    let key_path = vault_dir.join(".cryptokey");
    let crypto_key = crate::crypto::generate_key();
    std::fs::write(&key_path, crate::crypto::key_to_base64(&crypto_key))
        .map_err(|e| format!("Failed to write crypto key: {}", e))?;
    println!("[vault] Generated new crypto key for new vault: {:?}", vault_dir);

    let config = VaultConfig {
        path: db_path.to_string_lossy().to_string(),
    };
    save_vault_config(&vault_dir.to_string_lossy(), &config)?;

    // 注：雷达数据库在系统目录，不需要在新仓库创建时初始化
    // 雷达数据库会在首次访问时在系统目录自动创建

    let mut index = load_vault_index(&app);
    index.vaults.push(VaultInfo {
        name: name.clone(),
        path: vault_dir.to_string_lossy().to_string(),
    });
    save_vault_index(&app, &index)?;

    // 切回原仓库数据库和密钥
    if let Some(orig_db_path) = current_db_path {
        if orig_db_path.exists() {
            crate::db::switch_database(orig_db_path).map_err(|e| e.to_string())?;
        }
    }
    if let Some(orig_key) = current_crypto_key {
        crate::crypto::init_crypto(orig_key);
    }

    Ok(Vault {
        name,
        path: vault_dir.to_string_lossy().to_string(),
        project_count: 0,
    })
}

pub fn list_vaults(app: AppHandle) -> Result<Vec<Vault>, String> {
    let index = load_vault_index(&app);
    let mut vaults = Vec::new();

    for vault_info in &index.vaults {
        let db_path = PathBuf::from(&vault_info.path).join("os_compass.db");
        let project_count = if db_path.exists() {
            if let Ok(conn) = Connection::open(&db_path) {
                conn.query_row(
                    "SELECT COUNT(*) FROM projects WHERE data_status = 'ACTIVE'",
                    [],
                    |row| row.get::<_, i64>(0),
                ).unwrap_or(0)
            } else {
                0
            }
        } else {
            0
        };

        vaults.push(Vault {
            name: vault_info.name.clone(),
            path: vault_info.path.clone(),
            project_count,
        });
    }

    Ok(vaults)
}

pub fn migrate_to_vault(app: AppHandle, vault_path: String, old_db_path: PathBuf) -> Result<i64, String> {
    if !old_db_path.exists() {
        return Ok(0);
    }

    let vault_dir = PathBuf::from(&vault_path);
    let new_db_path = vault_dir.join("os_compass.db");

    let old_conn = Connection::open(&old_db_path).map_err(|e| e.to_string())?;
    let mut new_conn = Connection::open(&new_db_path).map_err(|e| e.to_string())?;

    let tables = ["categories", "tags", "project_tags", "projects", "translations", "project_releases", "project_notes"];

    for table in tables {
        let count: i64 = old_conn.query_row(
            &format!("SELECT COUNT(*) FROM {}", table),
            [],
            |row| row.get(0),
        ).unwrap_or(0);

        if count > 0 {
            let copy_sql = format!("ATTACH DATABASE '{}' AS source; INSERT OR IGNORE INTO {table} SELECT * FROM source.{table}; DETACH DATABASE source;", old_db_path.to_string_lossy());
            new_conn.execute_batch(&copy_sql).map_err(|e| e.to_string())?;
        }
    }

    let project_count: i64 = new_conn.query_row(
        "SELECT COUNT(*) FROM projects WHERE data_status = 'ACTIVE'",
        [],
        |row| row.get(0),
    ).unwrap_or(0);

    Ok(project_count)
}

pub fn validate_vault(path: String) -> VaultValidation {
    let db_path = PathBuf::from(&path).join("os_compass.db");
    println!("[validate_vault] path={}, db_path={:?}, exists={}", path, db_path, db_path.exists());

    if !db_path.exists() {
        return VaultValidation {
            valid: false,
            error: Some(format!("数据库文件不存在: {:?}", db_path)),
            table_count: None,
            project_count: None,
        };
    }

    match Connection::open(&db_path) {
        Ok(conn) => {
            let table_count: i64 = conn.query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table'",
                [],
                |row| row.get(0),
            ).unwrap_or(0);
            println!("[validate_vault] table_count={}", table_count);

            if table_count == 0 {
                return VaultValidation {
                    valid: false,
                    error: Some("数据库无表结构，可能是初始化失败".to_string()),
                    table_count: Some(0),
                    project_count: None,
                };
            }

            // 检查 projects 表是否存在
            let has_projects: bool = conn.query_row(
                "SELECT COUNT(*) > 0 FROM sqlite_master WHERE type='table' AND name='projects'",
                [],
                |row| row.get(0),
            ).unwrap_or(false);

            let project_count = if has_projects {
                conn.query_row(
                    "SELECT COUNT(*) FROM projects WHERE data_status = 'ACTIVE'",
                    [],
                    |row| row.get::<_, i64>(0),
                ).unwrap_or(0)
            } else {
                0
            };
            println!("[validate_vault] has_projects={}, project_count={}", has_projects, project_count);

            VaultValidation {
                valid: true,
                error: None,
                table_count: Some(table_count),
                project_count: Some(project_count),
            }
        }
        Err(e) => {
            println!("[validate_vault] Failed to open db: {}", e);
            VaultValidation {
                valid: false,
                error: Some(format!("无法打开数据库: {}", e)),
                table_count: None,
                project_count: None,
            }
        }
    }
}

pub fn open_vault(app: AppHandle, path: String) -> Result<Vault, String> {
    println!("[open_vault] Opening vault at path: {}", path);

    let validation = validate_vault(path.clone());
    println!("[open_vault] Validation: valid={}, error={:?}", validation.valid, validation.error);

    if !validation.valid {
        return Err(validation.error.unwrap_or_else(|| "仓库无效".to_string()));
    }

    let db_path = PathBuf::from(&path).join("os_compass.db");
    let db_path_str = db_path.to_string_lossy().to_string();
    println!("[open_vault] Switching database to: {}", db_path_str);

    crate::db::switch_database(db_path).map_err(|e| {
        println!("[open_vault] switch_database failed: {}", e);
        e.to_string()
    })?;
    println!("[open_vault] Database switched successfully");

    let config = VaultConfig {
        path: db_path_str,
    };
    {
        let mut current_config = CURRENT_VAULT_CONFIG.lock().unwrap();
        *current_config = Some(config);
    }

    // 注：雷达数据存储在系统目录，不需要在切换仓库时初始化
    // 信息源在系统库（radar_sources），采集数据在系统目录（radar_items）

    let last_vault_path = get_last_vault_path(&app);
    if let Some(parent) = last_vault_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    std::fs::write(&last_vault_path, &path).map_err(|e| e.to_string())?;

    let vault_name = std::path::Path::new(&path)
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "未知".to_string());

    let project_count = validation.project_count.unwrap_or(0);

    Ok(Vault {
        name: vault_name,
        path,
        project_count,
    })
}

pub fn delete_vault(app: AppHandle, path: String, permanent: bool) -> Result<(), String> {
    let mut index = load_vault_index(&app);
    index.vaults.retain(|v| v.path != path);
    save_vault_index(&app, &index)?;

    if permanent {
        let vault_dir = PathBuf::from(&path);
        if vault_dir.exists() {
            std::fs::remove_dir_all(&vault_dir).map_err(|e| e.to_string())?;
        }
    }

    Ok(())
}

pub fn get_current_vault(app: AppHandle) -> Result<Option<Vault>, String> {
    let last_vault_path = get_last_vault_path(&app);
    if !last_vault_path.exists() {
        return Ok(None);
    }

    let path = std::fs::read_to_string(&last_vault_path).map_err(|e| e.to_string())?;
    let path = path.trim().to_string();

    let index = load_vault_index(&app);
    if let Some(vault_info) = index.vaults.iter().find(|v| v.path == path) {
        let validation = validate_vault(path.clone());
        Ok(Some(Vault {
            name: vault_info.name.clone(),
            path: vault_info.path.clone(),
            project_count: validation.project_count.unwrap_or(0),
        }))
    } else {
        Ok(None)
    }
}

pub fn get_current_config() -> Option<VaultConfig> {
    let config = CURRENT_VAULT_CONFIG.lock().unwrap();
    config.clone()
}

pub fn save_current_config(config: VaultConfig) -> Result<(), String> {
    let last_vault_path = std::env::var("CURRENT_VAULT_PATH")
        .unwrap_or_default();
    if last_vault_path.is_empty() {
        return Err("当前没有打开的仓库".to_string());
    }
    save_vault_config(&last_vault_path, &config)?;
    let mut current = CURRENT_VAULT_CONFIG.lock().unwrap();
    *current = Some(config);
    Ok(())
}

#[derive(Debug)]
pub struct VaultIntegrityInfo {
    pub project_count: i64,
}

fn check_vault_integrity(path: &str) -> Result<VaultIntegrityInfo, String> {
    use std::collections::HashSet;

    let vault_dir = PathBuf::from(path);

    if !vault_dir.exists() {
        return Err("目录不存在".to_string());
    }

    let db_path = vault_dir.join("os_compass.db");
    if !db_path.exists() {
        return Err("未找到 os_compass.db，不是有效的仓库目录".to_string());
    }

    let key_path = vault_dir.join(".cryptokey");
    if !key_path.exists() {
        return Err("未找到 .cryptokey 加密密钥文件，无法解密敏感数据".to_string());
    }

    let conn = Connection::open(&db_path)
        .map_err(|e| format!("数据库文件损坏：{}", e))?;

    let required_tables: Vec<&str> = vec![
        "categories", "tags", "projects", "project_tags", "project_notes",
        "readme_variants", "translations", "project_clone", "project_releases",
    ];

    let mut stmt = conn
        .prepare("SELECT name FROM sqlite_master WHERE type='table'")
        .map_err(|e| e.to_string())?;
    let existing_tables: HashSet<String> = stmt
        .query_map([], |row| row.get(0))
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    let missing: Vec<&str> = required_tables
        .iter()
        .filter(|t| !existing_tables.contains(&t.to_string()))
        .map(|t| *t)
        .collect();
    if !missing.is_empty() {
        return Err(format!(
            "数据库结构不完整，缺少表：{}。可能不是 OS-Compass 仓库或为旧版本",
            missing.join(", ")
        ));
    }

    let key_cols = [
        ("tags", vec!["source"]),
        ("projects", vec!["data_status", "lifecycle_status"]),
    ];
    for (table, cols) in &key_cols {
        let pragma_sql = format!("PRAGMA table_info({})", table);
        let mut p = conn.prepare(&pragma_sql).map_err(|e| e.to_string())?;
        let existing_cols: HashSet<String> = p
            .query_map([], |row| row.get::<_, String>(1))
            .map_err(|e| e.to_string())?
            .filter_map(|r| r.ok())
            .collect();
        for col in cols.clone() {
            if !existing_cols.contains(&col.to_string()) {
                return Err("数据库版本过旧，不支持导入".to_string());
            }
        }
    }

    let key_content = std::fs::read_to_string(&key_path)
        .map_err(|e| format!("读取 .cryptokey 失败：{}", e))?;
    let crypto_key = crate::crypto::key_from_base64(key_content.trim())
        .map_err(|_| ".cryptokey 文件格式无效，无法解密敏感数据".to_string())?;

    // V2.0: system_variables 已移到系统库，仓库库不再存储敏感数据

    let project_count = conn
        .query_row(
            "SELECT COUNT(*) FROM projects WHERE data_status = 'ACTIVE'",
            [],
            |row| row.get::<_, i64>(0),
        )
        .unwrap_or(0);

    Ok(VaultIntegrityInfo { project_count })
}

pub fn import_vault(app: AppHandle, name: String, path: String) -> Result<Vault, String> {
    let integrity = check_vault_integrity(&path)?;

    let mut index = load_vault_index(&app);

    if index.vaults.iter().any(|v| v.path == path) {
        return Err("该仓库已在清单中，无需重复导入".to_string());
    }

    if index.vaults.iter().any(|v| v.name == name) {
        return Err(format!("仓库名 '{}' 已存在，请修改名称", name));
    }

    index.vaults.push(VaultInfo {
        name: name.clone(),
        path: path.clone(),
    });
    save_vault_index(&app, &index)?;

    Ok(Vault {
        name,
        path,
        project_count: integrity.project_count,
    })
}


