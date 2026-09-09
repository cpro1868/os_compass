use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{Read, Write};
use std::path::PathBuf;
use zip::write::FileOptions;
use zip::ZipArchive;

#[derive(Debug, Serialize, Deserialize)]
pub struct BackupFile {
    pub name: String,
    pub path: String,
    pub size: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RestoreResult {
    pub vault_name: String,
    pub verified: Vec<String>,
    pub warnings: Vec<String>,
}

fn zip_directory(dir: &PathBuf, output: &PathBuf) -> Result<(), String> {
    let file = fs::File::create(output)
        .map_err(|e| format!("Failed to create zip file: {}", e))?;

    let mut zip = zip::ZipWriter::new(file);
    let options: FileOptions<'_, ()> = FileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated)
        .unix_permissions(0o755);

    walkdir::WalkDir::new(dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .for_each(|entry| {
            let path = entry.path();
            let name = path.strip_prefix(dir).unwrap_or(path);

            if path.is_file() {
                let _ = zip.start_file(name.to_string_lossy(), options.clone());
                if let Ok(mut file) = fs::File::open(path) {
                    let mut buffer = Vec::new();
                    let _ = file.read_to_end(&mut buffer);
                    let _ = zip.write_all(&buffer);
                }
            } else if path.is_dir() && name.to_string_lossy() != "" {
                let _ = zip.add_directory(name.to_string_lossy(), options.clone());
            }
        });

    zip.finish().map_err(|e| format!("Failed to finish zip: {}", e))?;
    Ok(())
}

fn unzip_to_directory(zip_path: &PathBuf, output_dir: &PathBuf) -> Result<RestoreResult, String> {
    let file = fs::File::open(zip_path)
        .map_err(|e| format!("Failed to open zip: {}", e))?;

    let mut archive = ZipArchive::new(file)
        .map_err(|e| format!("Failed to read zip: {}", e))?;

    let mut verified = Vec::new();
    let mut warnings = Vec::new();

    for i in 0..archive.len() {
        let mut file = archive.by_index(i)
            .map_err(|e| format!("Failed to read file in zip: {}", e))?;

        let outpath = match file.enclosed_name() {
            Some(path) => output_dir.join(path),
            None => continue,
        };

        if file.name().ends_with('/') {
            fs::create_dir_all(&outpath).ok();
        } else {
            if let Some(parent) = outpath.parent() {
                if !parent.exists() {
                    fs::create_dir_all(parent).ok();
                }
            }
            let mut outfile = fs::File::create(&outpath)
                .map_err(|e| format!("Failed to create file: {}", e))?;
            std::io::copy(&mut file, &mut outfile)
                .map_err(|e| format!("Failed to write file: {}", e))?;
        }

        verified.push(file.name().to_string());
    }

    let vault_name = output_dir
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "Unknown".to_string());

    Ok(RestoreResult {
        vault_name,
        verified,
        warnings,
    })
}

fn check_backup_contents(zip_path: &PathBuf) -> Result<(bool, bool), String> {
    let file = fs::File::open(zip_path)
        .map_err(|e| format!("Failed to open zip: {}", e))?;

    let mut archive = ZipArchive::new(file)
        .map_err(|e| format!("Failed to read zip: {}", e))?;

    let mut has_db = false;
    let mut has_key = false;

    for i in 0..archive.len() {
        let file = archive.by_index(i)
            .map_err(|e| format!("Failed to read: {}", e))?;

        let name = file.name();
        if name.contains("os_compass.db") {
            has_db = true;
        }
        if name.contains(".cryptokey") {
            has_key = true;
        }
    }

    Ok((has_db, has_key))
}

#[tauri::command]
pub fn create_backup(vault_path: String, filename: String, dest_dir: String) -> Result<String, String> {
    let vault = PathBuf::from(&vault_path);
    let dest = PathBuf::from(&dest_dir);

    if !vault.exists() {
        return Err(format!("Vault path does not exist: {}", vault_path));
    }

    if !dest.exists() {
        fs::create_dir_all(&dest)
            .map_err(|e| format!("Failed to create destination directory: {}", e))?;
    }

    let zip_path = dest.join(format!("{}.zip", filename));

    zip_directory(&vault, &zip_path)?;

    Ok(zip_path.to_string_lossy().to_string())
}

#[tauri::command]
pub fn list_backups(dir: String) -> Result<Vec<BackupFile>, String> {
    let path = PathBuf::from(&dir);

    if !path.exists() {
        return Ok(vec![]);
    }

    let mut backups = Vec::new();

    let entries = fs::read_dir(&path)
        .map_err(|e| format!("Failed to read directory: {}", e))?;

    for entry in entries.flatten() {
        let file_path = entry.path();
        if file_path.extension().map(|e| e == "zip").unwrap_or(false) {
            let metadata = fs::metadata(&file_path)
                .map_err(|e| format!("Failed to get file metadata: {}", e))?;

            backups.push(BackupFile {
                name: file_path
                    .file_stem()
                    .map(|s| s.to_string_lossy().to_string())
                    .unwrap_or_default(),
                path: file_path.to_string_lossy().to_string(),
                size: metadata.len(),
            });
        }
    }

    backups.sort_by(|a, b| b.size.cmp(&a.size));
    Ok(backups)
}

#[tauri::command]
pub fn restore_backup(backup_path: String, target_dir: String) -> Result<RestoreResult, String> {
    let backup = PathBuf::from(&backup_path);
    let target = PathBuf::from(&target_dir);

    if !backup.exists() {
        return Err(format!("Backup file not found: {}", backup_path));
    }

    if target.exists() {
        let entries = fs::read_dir(&target)
            .map_err(|e| format!("Failed to read target directory: {}", e))?;
        
        if entries.count() > 0 {
            return Err("Target directory is not empty".to_string());
        }
    } else {
        fs::create_dir_all(&target)
            .map_err(|e| format!("Failed to create target directory: {}", e))?;
    }

    unzip_to_directory(&backup, &target)
}

#[tauri::command]
pub fn verify_backup(backup_path: String) -> Result<(bool, bool), String> {
    let backup = PathBuf::from(&backup_path);
    check_backup_contents(&backup)
}
