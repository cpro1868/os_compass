use crate::db::DATABASE;
use crate::system_db;
use std::path::PathBuf;
use rusqlite::Connection;

pub struct ConfigMigrator {
    vault_path: PathBuf,
    system_config_dir: PathBuf,
}

impl ConfigMigrator {
    pub fn new(vault_path: PathBuf, system_config_dir: PathBuf) -> Self {
        Self {
            vault_path,
            system_config_dir,
        }
    }

    pub fn needs_migration(&self) -> bool {
        !system_db::is_migration_needed()
    }

    pub fn migrate(&self) -> Result<MigrateResult, String> {
        let mut result = MigrateResult {
            settings_migrated: 0,
            tokens_migrated: 0,
            errors: Vec::new(),
        };

        let vault_db_path = self.vault_path.join("os_compass.db");
        if !vault_db_path.exists() {
            return Err("Vault database does not exist".to_string());
        }

        let conn = match Connection::open(&vault_db_path) {
            Ok(c) => c,
            Err(e) => return Err(format!("Failed to open vault database: {}", e)),
        };

        if let Err(e) = self.migrate_settings(&conn, &mut result) {
            result.errors.push(e);
        }

        if let Err(e) = self.migrate_variables(&conn, &mut result) {
            result.errors.push(e);
        }

        if result.errors.is_empty() {
            system_db::mark_migration_completed(None)
                .map_err(|e| format!("Failed to mark migration completed: {}", e))?;
        }

        Ok(result)
    }

    fn migrate_settings(&self, conn: &Connection, result: &mut MigrateResult) -> Result<(), String> {
        let mut stmt = conn
            .prepare("SELECT key, value, is_secret FROM app_settings WHERE key LIKE 'settings.%'")
            .map_err(|e| e.to_string())?;

        let rows = stmt
            .query_map([], |row| {
                let key: String = row.get(0)?;
                let value: String = row.get(1)?;
                let is_secret: bool = row.get::<_, i32>(2)? != 0;
                Ok((key, value, is_secret))
            })
            .map_err(|e| e.to_string())?;

        for row_result in rows {
            match row_result {
                Ok((key, value, is_secret)) => {
                    let store_value = if is_secret {
                        match crate::crypto::decrypt_string(&value) {
                            Ok(plain) => {
                                if let Err(e) = system_db::set_system_setting(&key, &plain, true) {
                                    result.errors.push(format!("Failed to set '{}': {}", key, e));
                                    continue;
                                }
                                result.tokens_migrated += 1;
                                continue;
                            }
                            Err(e) => {
                                result.errors.push(format!("Failed to decrypt '{}': {}", key, e));
                                continue;
                            }
                        }
                    } else {
                        value.clone()
                    };

                    if let Err(e) = system_db::set_system_setting(&key, &store_value, false) {
                        result.errors.push(format!("Failed to set '{}': {}", key, e));
                    } else {
                        result.settings_migrated += 1;
                    }
                }
                Err(e) => {
                    result.errors.push(format!("Query error: {}", e));
                }
            }
        }

        Ok(())
    }

    fn migrate_variables(&self, conn: &Connection, result: &mut MigrateResult) -> Result<(), String> {
        let mut stmt = conn
            .prepare("SELECT key, value, is_secret FROM system_variables WHERE 1=1")
            .map_err(|e| e.to_string())?;

        let rows = stmt
            .query_map([], |row| {
                let key: String = row.get(0)?;
                let value: String = row.get(1)?;
                let is_secret: bool = row.get::<_, i32>(2)? != 0;
                Ok((key, value, is_secret))
            })
            .map_err(|e| e.to_string())?;

        for row_result in rows {
            match row_result {
                Ok((key, value, is_secret)) => {
                    let store_value = if is_secret && !value.is_empty() {
                        match crate::crypto::decrypt_string(&value) {
                            Ok(plain) => {
                                if let Err(e) = system_db::set_system_variable(&key, &plain, true) {
                                    result.errors.push(format!("Failed to set variable '{}': {}", key, e));
                                    continue;
                                }
                                result.tokens_migrated += 1;
                                continue;
                            }
                            Err(e) => {
                                result.errors.push(format!("Failed to decrypt variable '{}': {}", key, e));
                                continue;
                            }
                        }
                    } else {
                        value.clone()
                    };

                    if let Err(e) = system_db::set_system_variable(&key, &store_value, is_secret) {
                        result.errors.push(format!("Failed to set variable '{}': {}", key, e));
                    } else {
                        if is_secret {
                            result.tokens_migrated += 1;
                        } else {
                            result.settings_migrated += 1;
                        }
                    }
                }
                Err(e) => {
                    result.errors.push(format!("Query error: {}", e));
                }
            }
        }

        Ok(())
    }
}

#[derive(Debug, serde::Serialize)]
pub struct MigrateResult {
    pub settings_migrated: usize,
    pub tokens_migrated: usize,
    pub errors: Vec<String>,
}

pub fn migrate_from_vault(vault_path: &str) -> Result<MigrateResult, String> {
    let system_config_dir = crate::system_db::get_system_settings_path(&PathBuf::from(
        directories::BaseDirs::new()
            .map(|d| d.data_dir().to_path_buf())
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".os-compass"),
    ));

    let migrator = ConfigMigrator::new(PathBuf::from(vault_path), system_config_dir);

    if !migrator.needs_migration() {
        return Err("Migration already completed".to_string());
    }

    migrator.migrate()
}

pub fn check_vault_needs_migration(vault_path: &str) -> bool {
    let vault_db_path = PathBuf::from(vault_path).join("os_compass.db");
    if !vault_db_path.exists() {
        return false;
    }

    let conn = match Connection::open(&vault_db_path) {
        Ok(c) => c,
        Err(_) => return false,
    };

    let settings_count: i32 = conn
        .query_row(
            "SELECT COUNT(*) FROM app_settings WHERE key LIKE 'settings.%'",
            [],
            |row| row.get(0),
        )
        .unwrap_or(0);

    let vars_count: i32 = conn
        .query_row(
            "SELECT COUNT(*) FROM system_variables WHERE value != ''",
            [],
            |row| row.get(0),
        )
        .unwrap_or(0);

    settings_count > 0 || vars_count > 0
}
