use crate::services::article_service::ArticleService;
use tauri_plugin_dialog::DialogExt;
use log;

#[tauri::command]
pub async fn generate_article(
    project_id: i64,
    style: String,
    word_count: String,
) -> Result<String, String> {
    ArticleService::generate(project_id, &style, &word_count).await
}

#[tauri::command]
pub async fn export_article(
    app: tauri::AppHandle,
    content: String,
    filename: String,
    format: String,
) -> Result<String, String> {
    log::info!("[Article] Exporting article as {} to {}", format, filename);
    
    let default_ext = match format.as_str() {
        "markdown" => "md",
        "pdf" => "pdf",
        "txt" => "txt",
        _ => return Err("不支持的格式".to_string()),
    };
    
let save_path = app.dialog()
        .file()
        .set_file_name(&format!("{}.{}", filename, default_ext))
        .add_filter("All Files", &["*"])
        .add_filter("Markdown", &["md"])
        .blocking_save_file();
    
    match save_path {
        Some(path) => {
            let path_str = path.to_string();
            std::fs::write(&path_str, &content)
                .map_err(|e| format!("保存文件失败: {}", e))?;
            log::info!("[Article] Article exported successfully to {}", path_str);
            Ok(path_str)
        }
        None => Err("用户取消保存".to_string()),
    }
}

#[tauri::command]
pub fn get_article_styles() -> Vec<ArticleStyle> {
    vec![
        ArticleStyle {
            id: "tech_popular".to_string(),
            name: "技术科普".to_string(),
            description: "通俗易懂，适合非技术背景读者".to_string(),
        },
        ArticleStyle {
            id: "business".to_string(),
            name: "商业推广".to_string(),
            description: "专业严谨，突出商业价值".to_string(),
        },
        ArticleStyle {
            id: "tech_blog".to_string(),
            name: "技术博客".to_string(),
            description: "技术深度与可读性并重".to_string(),
        },
        ArticleStyle {
            id: "brief".to_string(),
            name: "简介说明".to_string(),
            description: "简洁明了，信息密度高".to_string(),
        },
        ArticleStyle {
            id: "humor".to_string(),
            name: "幽默风趣".to_string(),
            description: "轻松愉快，适合社交媒体".to_string(),
        },
    ]
}

#[derive(serde::Serialize)]
pub struct ArticleStyle {
    pub id: String,
    pub name: String,
    pub description: String,
}
