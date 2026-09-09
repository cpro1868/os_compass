use rusqlite::Connection;

pub struct ArticlePrompts;

impl ArticlePrompts {
    pub fn init_system_db(conn: &Connection) -> Result<(), String> {
        conn.execute(
            "CREATE TABLE IF NOT EXISTS article_prompts (
                style VARCHAR(50) PRIMARY KEY,
                prompt TEXT NOT NULL
            )",
            [],
        ).map_err(|e| e.to_string())?;

        Self::insert_default_templates(conn)?;
        Ok(())
    }

    fn insert_default_templates(conn: &Connection) -> Result<(), String> {
        use crate::prompts::*;
        let templates = vec![
            ("tech_popular", TECH_POPULAR),
            ("business", BUSINESS),
            ("tech_blog", TECH_BLOG),
            ("brief", BRIEF),
            ("humor", HUMOR),
        ];

        for (style, prompt) in templates {
            conn.execute(
                "INSERT OR REPLACE INTO article_prompts (style, prompt) VALUES (?1, ?2)",
                [style, prompt],
            ).map_err(|e| e.to_string())?;
        }
        Ok(())
    }

    pub fn get_prompt(conn: &Connection, style: &str) -> Result<String, String> {
        conn.query_row(
            "SELECT prompt FROM article_prompts WHERE style = ?1",
            [style],
            |row| row.get(0),
        ).map_err(|e| format!("未找到风格 {} 的 Prompt", style))
    }

    pub fn get_all_styles(conn: &Connection) -> Result<Vec<(String, String)>, String> {
        let mut stmt = conn.prepare("SELECT style, prompt FROM article_prompts")
            .map_err(|e| e.to_string())?;
        
        let styles = stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        }).map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();
        
        Ok(styles)
    }
}
