use crate::db::DATABASE;
use crate::llm::{LlmClient, LlmMessage};

#[tauri::command]
pub async fn generate_runbook(id: i64) -> Result<String, String> {
    let readme_content = {
        let db = DATABASE.lock().map_err(|e| e.to_string())?;
        let db = db.as_ref().ok_or("Database not initialized")?;
        let conn = db.get_connection();
        
        let (readme, name): (Option<String>, String) = conn
            .query_row(
                "SELECT readme_content, name FROM projects WHERE id = ?",
                [id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .map_err(|e| e.to_string())?;
        (readme, name)
    };

    let llm = LlmClient::from_settings()
        .ok_or_else(|| "LLM not configured".to_string())?;

    let prompt = format!(
        r#"Based on the following README from the project "{}", generate a practical quick-start guide (Runbook) in Chinese.

The Runbook should include:
1. Project overview (1-2 sentences)
2. Prerequisites (what you need before starting)
3. Installation steps
4. How to run the project
5. Common commands
6. Troubleshooting tips

README:
{}

Return the Runbook in a clear, structured format using Chinese."#,
        readme_content.1,
        readme_content.0.as_deref().map(|r| crate::llm::preprocess_readme_for_runbook(r)).unwrap_or("No README available".to_string())
    );

    let messages = vec![
        LlmMessage {
            role: "system".to_string(),
            content: "You are an expert technical writer specializing in open source projects. Generate clear, practical Runbooks in Chinese.".to_string(),
        },
        LlmMessage {
            role: "user".to_string(),
            content: prompt,
        },
    ];

    llm.chat(messages).await
}
