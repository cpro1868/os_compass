pub mod article;
pub mod commands;
pub mod content_filter;
pub mod crypto;
pub mod crawler_service;
pub mod db;
pub mod embedding;
pub mod error;
pub mod feature_plugin;
pub mod health;
pub mod llm;
pub mod migrator;
pub mod models;
pub mod plugin_config_db;
pub mod plugin_loader;
pub mod plugin_manager;
pub mod plugins;
pub mod services;
pub mod settings;
pub mod source_engine;
pub mod system_db;
pub mod translate;
pub mod vault;

mod prompts;

#[cfg(test)]
mod db_tests;

use std::sync::Mutex;
use tauri::Manager;
use tauri::{command, Emitter};
use log;

lazy_static::lazy_static! {
    pub static ref PROJECT_DIR: Mutex<Option<std::path::PathBuf>> = Mutex::new(None);
}

#[command]
fn get_app_data_dir(app: tauri::AppHandle) -> String {
    app.path().app_data_dir()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_default()
}

#[command]
fn get_radar_data_dir() -> String {
    if let Some(base_dirs) = directories::BaseDirs::new() {
        base_dirs.data_dir().join(".os-compass")
            .to_string_lossy()
            .replace('\\', "/")
    } else {
        String::new()
    }
}

#[command]
fn get_vault_dir() -> String {
    PROJECT_DIR.lock().unwrap()
        .as_ref()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_default()
}

fn find_git_root(path: &std::path::Path) -> Option<std::path::PathBuf> {
    let mut current = path;
    loop {
        if current.join(".git").exists() {
            return Some(current.to_path_buf());
        }
        current = current.parent()?;
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    log::info!("OS-Compass starting...");
    
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.set_focus();
                let _ = window.unminimize();
            }
        }))
        .plugin(
            tauri_plugin_log::Builder::new()
                .target(tauri_plugin_log::Target::new(
                    tauri_plugin_log::TargetKind::LogDir { file_name: Some("os-compass".into()) },
                ))
                .rotation_strategy(tauri_plugin_log::RotationStrategy::KeepAll)
                .build(),
        )
        .setup(|app| {
            use tauri::menu::{MenuBuilder, MenuItemBuilder};
            use tauri::tray::TrayIconBuilder;

            let show_item = MenuItemBuilder::with_id("show", "显示/隐藏").build(app)?;
            let quit_item = MenuItemBuilder::with_id("quit", "退出").build(app)?;

            let menu = MenuBuilder::new(app)
                .items(&[&show_item, &quit_item])
                .build()?;

            let _tray = TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .menu(&menu)
                .tooltip("OS-Compass")
                .on_menu_event(|app, event| {
                    match event.id().as_ref() {
                        "show" => {
                            if let Some(window) = app.get_webview_window("main") {
                                if window.is_visible().unwrap_or(false) {
                                    let _ = window.hide();
                                } else {
                                    let _ = window.show();
                                    let _ = window.set_focus();
                                }
                            }
                        }
                        "quit" => {
                            app.exit(0);
                        }
                        _ => {}
                    }
                })
                .build(app)?;

            // 拦截窗口关闭事件，最小化到托盘
            let window = app.get_webview_window("main").unwrap();
            let app_handle = app.handle().clone();
            window.on_window_event(move |event| {
                if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                    api.prevent_close();
                    if let Some(w) = app_handle.get_webview_window("main") {
                        let _ = w.hide();
                    }
                }
            });

            // 系统库：Roaming 下的 .os-compass
            let app_dir = app.path().app_data_dir().expect("Failed to get app data dir");
            log::info!("App data directory (system): {:?}", app_dir);
            std::fs::create_dir_all(&app_dir).expect("Failed to create app data dir");

            if let Err(e) = system_db::init_system_db(&app_dir) {
                log::error!("[system_db] Failed to initialize system DB: {}", e);
            } else {
                log::info!("[system_db] System DB initialized successfully");
                if let Ok(system_db) = system_db::SYSTEM_DB.lock() {
                    if let Some(db) = system_db.as_ref() {
                        let conn = db.get_connection();
                        if let Err(e) = article::ArticlePrompts::init_system_db(&conn) {
                            log::error!("[article_prompts] Failed to initialize: {}", e);
                        } else {
                            log::info!("[article_prompts] Article prompts initialized successfully");
                        }
                    }
                }
            }

            // 仓库库：Local 下的 com.administrator.os-compass
            let vault_root = app.path().app_local_data_dir().expect("Failed to get local data dir");
            log::info!("Vault root directory: {:?}", vault_root);
            std::fs::create_dir_all(&vault_root).expect("Failed to create vault root dir");
            
            let vaults_root = vault_root.join("vaults");
            let default_vault_dir = vaults_root.join("default");
            let default_vault_path = default_vault_dir.join("os_compass.db");

            // 迁移旧仓库到新位置（如果需要）
            let old_vault_root = app_dir.clone();
            let old_db_path = old_vault_root.join("os_compass.db");
            let old_key_path = old_vault_root.join(".cryptokey");
            let old_config_path = old_vault_root.join("config.json");
            
            if !default_vault_path.exists() && old_db_path.exists() {
                log::info!("[vault] Migrating vault from old location: {:?}", old_vault_root);
                std::fs::create_dir_all(&default_vault_dir).ok();
                
                // 复制仓库库文件
                if old_db_path.exists() {
                    std::fs::copy(&old_db_path, &default_vault_path).ok();
                    log::info!("[vault] Copied os_compass.db");
                }
                if old_key_path.exists() {
                    std::fs::copy(&old_key_path, &default_vault_dir.join(".cryptokey")).ok();
                    log::info!("[vault] Copied .cryptokey");
                }
                if old_config_path.exists() {
                    std::fs::copy(&old_config_path, &default_vault_dir.join("config.json")).ok();
                    log::info!("[vault] Copied config.json");
                }
            }

            let mut vault_path_loaded = false;
            let mut current_vault_dir: Option<std::path::PathBuf> = None;

            // last-vault.txt 在系统库（Roaming）
            let last_vault_path = app_dir.join("last-vault.txt");
            if last_vault_path.exists() {
                if let Ok(path) = std::fs::read_to_string(&last_vault_path) {
                    let path = path.trim().to_string();
                    if !path.is_empty() {
                        let vault_dir = std::path::PathBuf::from(&path);
                        let db_path = vault_dir.join("os_compass.db");
                        if db_path.exists() {
                            log::info!("Auto-opening last vault: {:?}", path);
                            if db::switch_database(db_path).is_ok() {
                                let config = vault::load_vault_config(&path);
                                let mut current = vault::CURRENT_VAULT_CONFIG.lock().unwrap();
                                *current = Some(config);
                                vault_path_loaded = true;
                                current_vault_dir = Some(vault_dir);
                            }
                        }
                    }
                }
            }

            if !vault_path_loaded && default_vault_path.exists() {
                log::info!("Loading existing database as default vault");
                if db::switch_database(default_vault_path.clone()).is_ok() {
                    let vault_name = "默认仓库".to_string();

                    let config = vault::VaultConfig {
                        path: default_vault_path.to_string_lossy().to_string(),
                    };

                    let mut current = vault::CURRENT_VAULT_CONFIG.lock().unwrap();
                    *current = Some(config);

                    let vault_info = vault::VaultInfo {
                        name: vault_name.clone(),
                        path: default_vault_dir.to_string_lossy().to_string(),
                    };
                    let app_handle = app.handle();
                    let mut index = vault::load_vault_index(&app_handle);
                    let new_vault_path = default_vault_dir.to_string_lossy().to_string();
                    
                    // 更新已有仓库的路径（如果有旧的仓库指向旧位置）
                    for vault in &mut index.vaults {
                        if vault.path == old_vault_root.to_string_lossy().to_string() {
                            vault.path = new_vault_path.clone();
                            log::info!("[vault] Updated vault path in index");
                        }
                    }
                    vault::save_vault_index(&app_handle, &index).ok();
                    
                    if !index.vaults.iter().any(|v| v.path == new_vault_path) {
                        index.vaults.push(vault_info);
                        vault::save_vault_index(&app_handle, &index).ok();
                    }

                    let last_vault_txt = app_dir.join("last-vault.txt");
                    std::fs::write(&last_vault_txt, default_vault_dir.to_string_lossy().as_ref()).ok();

                    vault_path_loaded = true;
                    current_vault_dir = Some(default_vault_dir);
                }
            }

            if let Ok(exe_path) = std::env::current_exe() {
                if let Some(parent) = exe_path.parent() {
                    if let Some(git_root) = find_git_root(parent) {
                        let mut project_dir = PROJECT_DIR.lock().unwrap();
                        *project_dir = Some(git_root);
                    }
                    // 确保 scripts/init_schema.sql 存在于 exe 同级目录
                    let scripts_dir = parent.join("scripts");
                    let target_script = scripts_dir.join("init_schema.sql");
                    if !target_script.exists() {
                        std::fs::create_dir_all(&scripts_dir).ok();
                        // 优先从 Tauri 资源目录复制（安装版）
                        let mut copied = false;
                        if let Ok(resource_dir) = app.path().resource_dir() {
                            let res_script = resource_dir.join("init_schema.sql");
                            if res_script.exists() {
                                if std::fs::copy(&res_script, &target_script).is_ok() {
                                    copied = true;
                                    log::info!("Copied init_schema.sql from resource dir");
                                }
                            }
                        }
                        // 兜底：从项目 scripts 目录复制（开发模式）
                        if !copied {
                            if let Some(proj_dir) = PROJECT_DIR.lock().unwrap().as_ref() {
                                let dev_script = proj_dir.join("scripts").join("init_schema.sql");
                                if dev_script.exists() {
                                    if std::fs::copy(&dev_script, &target_script).is_ok() {
                                        log::info!("Copied init_schema.sql from project dir");
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // 加密密钥：系统级密钥，存储在 app_data_dir，不跟随仓库切换
            // 所有系统级配置（llm_api_key 等）使用此密钥加密
            let key_path = app_dir.join(".cryptokey");
            log::info!("Using system-level crypto key path: {:?}", key_path);
            let crypto_key = if key_path.exists() {
                match std::fs::read_to_string(&key_path) {
                    Ok(content) => {
                        match crypto::key_from_base64(content.trim()) {
                            Ok(k) => {
                                log::info!("Loaded system-level crypto key");
                                k
                            }
                            Err(_) => {
                                let k = crypto::generate_key();
                                let _ = std::fs::write(&key_path, crypto::key_to_base64(&k));
                                k
                            }
                        }
                    }
                    Err(_) => {
                        let k = crypto::generate_key();
                        let _ = std::fs::write(&key_path, crypto::key_to_base64(&k));
                        k
                    }
                }
            } else {
                // 兼容：从 vault 目录的 .cryptokey 迁移过来
                let vault_dir = current_vault_dir.clone()
                    .unwrap_or_else(|| app_dir.clone());
                let legacy_key_path = vault_dir.join(".cryptokey");
                if legacy_key_path.exists() {
                    if let Ok(content) = std::fs::read_to_string(&legacy_key_path) {
                        if let Ok(k) = crypto::key_from_base64(content.trim()) {
                            let _ = std::fs::write(&key_path, crypto::key_to_base64(&k));
                            log::info!("Migrated crypto key from vault dir to system dir");
                            crypto::init_crypto(k);
                            let _ = crate::settings::get_settings();
                            crate::commands::variables::cleanup_undecryptable_secrets();
                            log::info!("Database, settings, and crypto initialized (legacy vault key)");
                            return Ok(());
                        }
                    }
                }
                // 首次启动：生成新密钥
                let k = crypto::generate_key();
                let _ = std::fs::write(&key_path, crypto::key_to_base64(&k));
                log::info!("Generated new system-level crypto key");
                k
            };
            crypto::init_crypto(crypto_key);

            // 验证加密服务可用
            match crypto::encrypt_string("__verify__") {
                Ok(_) => log::info!("[crypto] Verification: encrypt/decrypt service OK"),
                Err(e) => log::warn!("[crypto] WARNING: encrypt verification failed: {}", e),
            }

            // 启动时立即触发一次 settings 读取，确保从 settings.db 迁移到 app_settings
            let _ = crate::settings::get_settings();

            // 检查 system_variables 中无法用当前密钥解密的旧密文（仅警告，不清空）
            crate::commands::variables::cleanup_undecryptable_secrets();

            log::info!("Database, settings, and crypto initialized");
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_projects,
            commands::get_project,
            commands::get_recent_projects,
            commands::get_project_preview,
            commands::create_project,
            commands::update_project,
            commands::update_project_status,
            commands::archive_project,
            commands::restore_project,
            commands::get_archived_projects,
            commands::batch_archive_projects,
            commands::batch_delete_projects,
            commands::delete_project,
            commands::batch_restore_projects,
            commands::batch_move_category,
            commands::get_categories,
            commands::get_category,
            commands::create_category,
            commands::update_category,
            commands::delete_category,
            commands::get_tags,
            commands::get_project_tags,
            commands::create_tag,
            commands::add_tag_to_project,
            commands::remove_tag_from_project,
            commands::delete_tag,
            commands::get_project_notes,
            commands::create_note,
            commands::update_note,
            commands::delete_note,
            commands::get_project_user_info,
            commands::add_project_user_info,
            commands::update_project_user_info,
            commands::delete_project_user_info,
            commands::copy_user_info_value,
            commands::import_project,
            commands::preview_import,
            commands::post_import_tasks,
            commands::get_settings,
            commands::save_settings,
            commands::analyze_project,
            commands::test_llm_direct,
            commands::debug_llm_status,
            commands::calculate_project_health,
            commands::crypto_status,
            commands::encrypt_data,
            commands::decrypt_data,
            commands::init_crypto_with_key,
            commands::generate_crypto_key,
            commands::derive_key,
            commands::test_encryption,
            commands::get_system_variables,
            commands::get_system_variable,
            commands::set_system_variable,
            commands::delete_system_variable,
            commands::get_variable,
            commands::list_extensions,
            commands::get_extension,
            commands::set_extension_enabled,
            commands::get_enabled_extensions,
            commands::get_supported_platform_domains,
            commands::get_supported_platforms,
            commands::translate,
            commands::translate_with_llm,
            commands::detect_text_language,
            commands::should_translate,
            commands::generate_runbook,
            commands::save_runbook,
            commands::get_runbook,
            commands::generate_tags,
            commands::get_llm_models,
            commands::save_translations,
            commands::get_translations,
            commands::debug_translations,
            commands::debug_translations_table,
            commands::save_readme_translation,
            commands::clear_translations,
            commands::test_llm_connection,
            commands::list_available_models,
            commands::test_translate,
            commands::test_readme,
            commands::refresh_project_readme,
            commands::get_readme_variants_cmd,
            commands::save_readme_variants_cmd,
            commands::get_translations_cmd,
            commands::save_translation_cmd,
            commands::delete_translation_cmd,
            commands::debug_all_translations,
            commands::get_stats,
            commands::get_clone_settings,
            commands::save_clone_settings,
            commands::execute_clone,
            commands::path_exists,
            commands::get_releases,
            commands::fetch_releases,
            commands::translate_release_body,
            commands::ai_classify_project,
            commands::get_vaults_root,
            commands::create_vault,
            commands::list_vaults,
            commands::open_vault,
            commands::delete_vault,
            commands::validate_vault,
            commands::get_current_vault,
            commands::get_vault_config,
            commands::save_vault_config,
            commands::migrate_old_data,
            commands::get_old_db_path,
            commands::import_vault,
            commands::open_folder,
            commands::list_feature_plugins,
            commands::set_plugin_enabled,
            commands::get_plugin_config,
            commands::save_plugin_config,
            commands::plugin_get_db_path,
            commands::list_radar_sources,
            commands::add_radar_source,
            commands::update_radar_source,
            commands::delete_radar_source,
            commands::get_radar_items,
            commands::radar_item_action,
            commands::trigger_radar_scan,
            commands::get_radar_unread_count,
            commands::clear_radar_cache,
            commands::clear_radar_all,
            commands::intent_search,
            commands::get_search_history,
            commands::clear_search_history,
            commands::list_search_sources,
            commands::add_search_source,
            commands::update_search_source,
            commands::delete_search_source,
            commands::refresh_search_cache,
            commands::import_search_result,
            commands::get_embedding_settings,
            commands::save_embedding_settings,
            commands::test_embedding_connection,
            commands::list_embedding_models,
            embedding::generate_embedding_for_text,
            embedding::generate_project_embeddings,
            embedding::rebuild_embeddings,
            embedding::debug_db_info,
            embedding::semantic_search_projects,
            commands::generate_article,
            commands::export_article,
            commands::get_article_styles,
            get_app_data_dir,
            get_radar_data_dir,
            get_vault_dir,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
