use crate::vault::{self, Vault, VaultConfig, VaultValidation};
use tauri::{command, AppHandle, Manager};

#[command]
pub fn create_vault(app: AppHandle, name: String, path: String) -> Result<Vault, String> {
    vault::create_vault(app, name, path)
}

#[command]
pub fn list_vaults(app: AppHandle) -> Result<Vec<Vault>, String> {
    vault::list_vaults(app)
}

#[command]
pub fn open_vault(app: AppHandle, path: String) -> Result<Vault, String> {
    vault::open_vault(app, path)
}

#[command]
pub fn delete_vault(app: AppHandle, path: String, permanent: bool) -> Result<(), String> {
    vault::delete_vault(app, path, permanent)
}

#[command]
pub fn validate_vault(path: String) -> VaultValidation {
    vault::validate_vault(path)
}

#[command]
pub fn get_current_vault(app: AppHandle) -> Result<Option<Vault>, String> {
    vault::get_current_vault(app)
}

#[command]
pub fn get_vaults_root(app: AppHandle) -> Result<String, String> {
    let data_dir = app.path().app_local_data_dir().map_err(|e| e.to_string())?;
    let vaults_root = data_dir.join("vaults");
    std::fs::create_dir_all(&vaults_root).map_err(|e| e.to_string())?;
    Ok(vaults_root.to_string_lossy().to_string())
}

#[command]
pub fn get_vault_config() -> Result<VaultConfig, String> {
    vault::get_current_config().ok_or_else(|| "当前没有打开的仓库".to_string())
}

#[command]
pub fn save_vault_config(config: VaultConfig) -> Result<(), String> {
    vault::save_current_config(config)
}

#[command]
pub fn migrate_old_data(app: AppHandle, vault_path: String, old_db_path: String) -> Result<i64, String> {
    vault::migrate_to_vault(app, vault_path, std::path::PathBuf::from(old_db_path))
}

#[command]
pub fn get_old_db_path(app: AppHandle) -> Result<String, String> {
    let data_dir = app.path().app_local_data_dir().map_err(|e| e.to_string())?;
    let old_path = data_dir.join("os_compass.db");
    if old_path.exists() {
        Ok(old_path.to_string_lossy().to_string())
    } else {
        Ok(String::new())
    }
}

#[command]
pub fn import_vault(app: AppHandle, name: String, path: String) -> Result<Vault, String> {
    vault::import_vault(app, name, path)
}

#[cfg(target_os = "windows")]
#[command]
pub fn open_folder(path: String) -> Result<(), String> {
    std::process::Command::new("explorer")
        .arg(&path)
        .spawn()
        .map_err(|e| format!("打开文件夹失败: {}", e))?;
    Ok(())
}

#[cfg(target_os = "macos")]
#[command]
pub fn open_folder(path: String) -> Result<(), String> {
    std::process::Command::new("open")
        .arg(&path)
        .spawn()
        .map_err(|e| format!("打开文件夹失败: {}", e))?;
    Ok(())
}

#[cfg(target_os = "linux")]
#[command]
pub fn open_folder(path: String) -> Result<(), String> {
    std::process::Command::new("xdg-open")
        .arg(&path)
        .spawn()
        .map_err(|e| format!("打开文件夹失败: {}", e))?;
    Ok(())
}
