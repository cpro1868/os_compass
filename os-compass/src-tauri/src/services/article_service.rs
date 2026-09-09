use crate::article::ArticlePrompts;
use crate::db::DATABASE;
use crate::llm::LlmClient;
use crate::system_db::SYSTEM_DB;
use log;

pub struct ArticleService;

impl ArticleService {
    pub async fn generate(
        project_id: i64,
        style: &str,
        word_count: &str,
    ) -> Result<String, String> {
        log::info!("[Article] Generating article for project {} with style={}, word_count={}", project_id, style, word_count);

        let (project_name, project_desc, readme, ai_use_cases) = {
            let db_lock = DATABASE.lock().map_err(|e| e.to_string())?;
            let db = db_lock.as_ref().ok_or("Database not initialized")?;
            let conn = db.get_connection();
            
            let name: String = conn.query_row(
                "SELECT COALESCE(name, '') FROM projects WHERE id = ?",
                rusqlite::params![project_id],
                |row| row.get(0),
            ).map_err(|e| format!("Failed to get project name: {}", e))?;
            
            let desc: String = conn.query_row(
                "SELECT COALESCE(description, '') FROM projects WHERE id = ?",
                rusqlite::params![project_id],
                |row| row.get(0),
            ).map_err(|e| format!("Failed to get project description: {}", e))?;
            
            let readme_content: String = conn.query_row(
                "SELECT COALESCE(readme_content, '') FROM projects WHERE id = ?",
                rusqlite::params![project_id],
                |row| row.get(0),
            ).map_err(|e| format!("Failed to get readme: {}", e))?;
            
            let use_cases: String = conn.query_row(
                "SELECT COALESCE(ai_use_cases, '暂无') FROM projects WHERE id = ?",
                rusqlite::params![project_id],
                |row| row.get(0),
            ).map_err(|e| format!("Failed to get AI use cases: {}", e))?;
            
            (name, desc, readme_content, use_cases)
        };
        
        let prompt_template = {
            let system_db = SYSTEM_DB.lock().map_err(|e| e.to_string())?;
            let db = system_db.as_ref().ok_or("System database not initialized")?;
            let conn = db.get_connection();
            ArticlePrompts::get_prompt(&conn, style)?
        };

        let prompt = Self::fill_variables(
            &prompt_template,
            &project_name,
            &project_desc,
            &ai_use_cases,
            &readme,
            word_count,
            style,
        )?;

        let llm = LlmClient::from_settings().ok_or("LLM not configured")?;
        let messages = vec![crate::llm::LlmMessage {
            role: "user".to_string(),
            content: prompt,
        }];
        let article = llm.chat(messages).await
            .map_err(|e| format!("LLM 调用失败: {}", e))?;

        log::info!("[Article] Article generated successfully, length={}", article.len());
        Ok(article)
    }

    fn get_project_name(project_id: i64) -> Result<String, String> {
        let db_lock = DATABASE.lock().map_err(|e| e.to_string())?;
        let db = db_lock.as_ref().ok_or("Database not initialized")?;
        let conn = db.get_connection();
        
        conn.query_row(
            "SELECT name FROM projects WHERE id = ?",
            rusqlite::params![project_id],
            |row| row.get::<_, String>(0),
        ).map_err(|e| format!("Failed to get project name: {}", e))
    }

    fn get_project_description(project_id: i64) -> Result<String, String> {
        let db_lock = DATABASE.lock().map_err(|e| e.to_string())?;
        let db = db_lock.as_ref().ok_or("Database not initialized")?;
        let conn = db.get_connection();
        
        conn.query_row(
            "SELECT description FROM projects WHERE id = ?",
            rusqlite::params![project_id],
            |row| row.get::<_, Option<String>>(0),
        ).map_err(|e| format!("Failed to get project description: {}", e))
        .map(|opt| opt.unwrap_or_default())
    }

    fn get_readme(project_id: i64) -> Result<String, String> {
        let db_lock = DATABASE.lock().map_err(|e| e.to_string())?;
        let db = db_lock.as_ref().ok_or("Database not initialized")?;
        let conn = db.get_connection();
        
        conn.query_row(
            "SELECT readme_content FROM projects WHERE id = ?",
            rusqlite::params![project_id],
            |row| row.get::<_, Option<String>>(0),
        ).map_err(|e| format!("Failed to get readme: {}", e))
        .map(|opt| opt.unwrap_or_default())
    }

    fn get_ai_use_cases(project_id: i64) -> Result<String, String> {
        let db_lock = DATABASE.lock().map_err(|e| e.to_string())?;
        let db = db_lock.as_ref().ok_or("Database not initialized")?;
        let conn = db.get_connection();
        
        conn.query_row(
            "SELECT ai_use_cases FROM projects WHERE id = ?",
            rusqlite::params![project_id],
            |row| row.get::<_, Option<String>>(0),
        ).map_err(|e| format!("Failed to get AI use cases: {}", e))
        .map(|opt| opt.unwrap_or_else(|| "暂无".to_string()))
    }

    fn fill_variables(
        template: &str,
        project_name: &str,
        project_desc: &str,
        use_cases: &str,
        readme: &str,
        word_count: &str,
        style: &str,
    ) -> Result<String, String> {
        let readme_excerpt = if readme.len() > 2000 {
            format!("{}...", &readme[..2000])
        } else {
            readme.to_string()
        };
        
        let style_name = match style {
            "tech_popular" => "技术科普",
            "business" => "商业推广",
            "tech_blog" => "技术博客",
            "brief" => "简介说明",
            "humor" => "幽默风趣",
            _ => style,
        };
        
        let word_count_text = match word_count {
            "short" => "500-800",
            "medium" => "1000-1500",
            "long" => "2000-3000",
            _ => word_count,
        };

        let final_prompt = template
            .replace("{project_name}", project_name)
            .replace("{description}", project_desc)
            .replace("{use_cases}", use_cases)
            .replace("{readme_excerpt}", &readme_excerpt)
            .replace("{word_count}", word_count_text)
            .replace("{style}", style_name);
        
        let preview = if final_prompt.len() > 2000 {
            final_prompt[..2000].to_string()
        } else {
            final_prompt.clone()
        };
        log::info!("[Article] Generated prompt (len={}):\n{}", final_prompt.len(), preview);
        
        Ok(final_prompt)
    }
}
