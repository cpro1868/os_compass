use crate::db::open_db_at_path;
use crate::feature_plugin::{
    DbMode, FeaturePlugin, FeaturePluginType, FeatureResult, PluginContext, PluginError,
};
use crate::llm::{LlmClient, LlmMessage};
use crate::settings::get_settings;
use crate::source_engine::llm_parser::recommend_projects_with_llm;
use async_trait::async_trait;
use rusqlite::params;
use serde::{Deserialize, Serialize};
use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;

fn get_debug_log_path() -> PathBuf {
    if let Some(base_dirs) = directories::BaseDirs::new() {
        base_dirs.data_dir().join(".os-compass").join("debug.log")
    } else {
        PathBuf::from("debug.log")
    }
}

pub fn write_debug_log(msg: &str) {
    if let Ok(mut file) = OpenOptions::new()
        .create(true)
        .append(true)
        .open(get_debug_log_path())
    {
        let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f");
        let _ = writeln!(file, "[{}] {}", timestamp, msg);
    }
    eprintln!("{}", msg);
}

pub struct SearchPlugin;

impl SearchPlugin {
    pub fn new() -> Self {
        SearchPlugin
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntentAnalysis {
    pub intent: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub intent_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub keywords: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub questions: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub options: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmartRecommendation {
    pub categories: Vec<String>,
    pub tags: Vec<String>,
    pub suggestions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub query: String,
    pub local_results: Vec<ProjectMatch>,
    pub web_results: Vec<ProjectMatch>,
    pub total: i64,
    pub conversation_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recommendation: Option<SmartRecommendation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub llm_text: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectMatch {
    pub name: String,
    pub url: String,
    pub description: Option<String>,
    pub stars: Option<i64>,
    pub forks: Option<i64>,
    pub language: Option<String>,
    pub health_score: Option<i64>,
    pub source: String,
    pub match_score: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project_id: Option<i64>,
}

/// 把 projects.languages 里存的 GitHub 多语言 JSON 数组串归一化为展示用的主语言
pub(crate) fn primary_language(raw: Option<String>) -> Option<String> {
    let raw = raw?.trim().to_string();
    if raw.is_empty() {
        return None;
    }
    if let Ok(serde_json::Value::Array(list)) = serde_json::from_str::<serde_json::Value>(&raw) {
        return list
            .iter()
            .filter_map(|v| v.as_str())
            .map(|s| s.trim().to_string())
            .find(|s| !s.is_empty());
    }
    Some(raw)
}

pub fn init_search_db(vault_dir: &std::path::Path) -> Result<rusqlite::Connection, String> {
    let db_path = vault_dir.join("plugin_search.db");
    let conn = open_db_at_path(&db_path)?;

    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS search_sources (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            source_type TEXT NOT NULL,
            url TEXT NOT NULL,
            platform TEXT,
            enabled INTEGER DEFAULT 1,
            created_at TEXT DEFAULT (datetime('now', 'localtime')),
            updated_at TEXT DEFAULT (datetime('now', 'localtime'))
        );

        CREATE TABLE IF NOT EXISTS search_cache (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            source_id INTEGER NOT NULL REFERENCES search_sources(id) ON DELETE CASCADE,
            url_hash TEXT NOT NULL UNIQUE,
            project_name TEXT,
            project_url TEXT,
            description TEXT,
            language TEXT,
            raw_content TEXT,
            keywords TEXT,
            fetched_at TEXT DEFAULT (datetime('now', 'localtime')),
            expires_at TEXT
        );

        CREATE TABLE IF NOT EXISTS search_history (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            query TEXT NOT NULL,
            result_count INTEGER DEFAULT 0,
            result_summary TEXT,
            created_at TEXT DEFAULT (datetime('now', 'localtime'))
        );

        CREATE INDEX IF NOT EXISTS idx_search_sources_enabled ON search_sources(enabled);
        CREATE INDEX IF NOT EXISTS idx_search_cache_keywords ON search_cache(keywords);
        CREATE INDEX IF NOT EXISTS idx_search_history_created ON search_history(created_at DESC);
        "#,
    )
    .map_err(|e| e.to_string())?;

    Ok(conn)
}

pub async fn three_layer_search(
    vault_dir: &std::path::Path,
    query: &str,
) -> Result<SearchResult, String> {
    write_debug_log(&format!("[DEBUG] three_layer_search: 开始，query={}, vault_dir={:?}", query, vault_dir));

    let search_conn = init_search_db(vault_dir)?;
    write_debug_log("[DEBUG] three_layer_search: search_conn 初始化完成");
    let settings = get_settings();
    write_debug_log("[DEBUG] three_layer_search: settings 加载完成");

    write_debug_log("[DEBUG] local_search: 开始向量搜索...");
    let mut local_results: Vec<ProjectMatch> = Vec::new();
    let local_search_error = if !crate::embedding::is_enabled() {
        Some("embedding 未启用".to_string())
    } else {
        match crate::embedding::semantic_search(query, 10).await {
            Ok(matches) => {
                let db_guard = crate::db::DATABASE.lock().unwrap();
                if let Some(db) = db_guard.as_ref() {
                    let main_conn = db.get_connection();
                    for (project_id, distance) in matches {
                        if let Ok((name, url, description, language, stars, forks)) = main_conn.query_row(
                            "SELECT name, url, description, languages, stars, forks FROM projects WHERE id = ?",
                            rusqlite::params![project_id],
                            |row| Ok((
                                row.get::<_, String>(0)?,
                                row.get::<_, Option<String>>(1)?.unwrap_or_default(),
                                row.get::<_, Option<String>>(2)?,
                                row.get::<_, Option<String>>(3)?,
                                row.get::<_, Option<i64>>(4)?,
                                row.get::<_, Option<i64>>(5)?,
                            )),
                        ) {
                            local_results.push(ProjectMatch {
                                name,
                                url,
                                description,
                                stars,
                                forks,
                                language: primary_language(language),
                                health_score: None,
                                source: "local".to_string(),
                                match_score: ((1.0f32 - distance).max(0.0f32) as f64),
                                project_id: Some(project_id),
                            });
                        }
                    }
                }
                None
            }
            Err(e) => Some(e),
        }
    };

    write_debug_log(&format!("[DEBUG] local_search: 向量搜索完成，结果={}，错误={:?}", local_results.len(), local_search_error));

    let mut llm_text = None;
    if local_results.len() < 3 {
        write_debug_log(&format!("[DEBUG] local_search: 结果不足 3 条，调用 LLM 知识推荐，当前={}，错误={:?}", local_results.len(), local_search_error));
        match recommend_projects_with_llm(query, &settings, 5).await {
            Ok(recommended) => {
                if !recommended.is_empty() {
                    llm_text = Some(format_llm_recommendations(&recommended));
                }
                write_debug_log(&format!("[DEBUG] llm_search: LLM 知识推荐完成，项目数={}", recommended.len()));
            }
            Err(e) => write_debug_log(&format!("[DEBUG] llm_search: LLM 知识推荐失败: {}", e)),
        }
    } else {
        write_debug_log("[DEBUG] local_search: 结果达到 3 条，不调用 LLM 推荐");
    }

    let web_results = Vec::new();
    let total = (local_results.len() + web_results.len()) as i64;
    let conversation_id = format!("conv_{}", std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis());

    search_conn.execute(
        "INSERT INTO search_history (query, result_count) VALUES (?, ?)",
        params![query, total],
    ).ok();

    log::info!("[three_layer_search] 生成本地推荐");
    let recommendation = generate_smart_recommendation(&local_results, &web_results, query);

    log::info!("[three_layer_search] ====== 搜索完成 ======");
    log::info!("[three_layer_search] 总结果数: {}", total);
    log::info!("[three_layer_search] 本地结果: {}, 联网结果: {}", local_results.len(), web_results.len());

    Ok(SearchResult {
        query: (*query).to_string(),
        local_results,
        web_results,
        total,
        conversation_id,
        recommendation: Some(recommendation),
        llm_text,
    })
}

fn format_llm_recommendations(projects: &[crate::source_engine::llm_parser::ParsedProjectInfo]) -> String {
    projects.iter().enumerate().map(|(index, project)| {
        let name = project.project_name.as_deref().unwrap_or("未命名项目");
        let url = project.project_url.as_deref().unwrap_or("暂无仓库地址");
        let description = project.description.as_deref().unwrap_or("暂无项目描述");
        let language = project.language.as_deref().unwrap_or("Unknown");
        format!("{}. **{}**\n   - GitHub/Gitee：{}\n   - 推荐理由：{}\n   - 主要语言：{}", index + 1, name, url, description, language)
    }).collect::<Vec<_>>().join("\n\n")
}

fn generate_smart_recommendation(
    local_results: &[ProjectMatch],
    web_results: &[ProjectMatch],
    query: &str,
) -> SmartRecommendation {
    let mut categories: Vec<String> = Vec::new();
    let mut tags: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut suggestions: Vec<String> = Vec::new();

    for result in local_results.iter().chain(web_results.iter()) {
        if let Some(lang) = &result.language {
            if !lang.is_empty() {
                tags.insert(lang.clone());
            }
        }
        if let Some(desc) = &result.description {
            let desc_lower = desc.to_lowercase();
            let words: Vec<&str> = desc_lower.split_whitespace().collect();
            for word in words.iter().take(5) {
                if word.len() > 3 && !tags.contains(*word) {
                    tags.insert(word.to_string());
                }
            }
        }
    }

    let query_lower = query.to_lowercase();
    if query_lower.contains("web") || query_lower.contains("前端") {
        categories.push("Web开发".to_string());
        tags.insert("javascript".to_string());
        tags.insert("typescript".to_string());
        suggestions.push("前端框架".to_string());
    }
    if query_lower.contains("ai") || query_lower.contains("机器学习") || query_lower.contains("ml") {
        categories.push("人工智能".to_string());
        tags.insert("python".to_string());
        tags.insert("深度学习".to_string());
        suggestions.push("深度学习框架".to_string());
    }
    if query_lower.contains("api") || query_lower.contains("后端") {
        categories.push("后端服务".to_string());
        suggestions.push("REST API".to_string());
    }

    let tags_vec: Vec<String> = tags.into_iter().take(8).collect();
    let suggestions_vec = if suggestions.is_empty() {
        vec![
            format!("{} 最佳实践", query),
            format!("{} 替代方案", query),
        ]
    } else {
        suggestions
    };

    SmartRecommendation {
        categories,
        tags: tags_vec,
        suggestions: suggestions_vec,
    }
}

fn generate_llm_summary(
    local_results: &[ProjectMatch],
    web_results: &[ProjectMatch],
    query: &str,
) -> String {
    let total_count = local_results.len() + web_results.len();
    
    if total_count == 0 {
        return format!(
            "针对「{}」的搜索未找到相关项目。\n\n建议：\n- 尝试使用更通用的关键词\n- 检查拼写是否正确\n- 尝试使用英文关键词",
            query
        );
    }
    
    let local_count = local_results.len();
    let web_count = web_results.len();
    
    let mut languages: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    let mut top_projects: Vec<&ProjectMatch> = Vec::new();
    
    for result in local_results.iter().chain(web_results.iter()) {
        if let Some(lang) = &result.language {
            if !lang.is_empty() {
                *languages.entry(lang.clone()).or_insert(0) += 1;
            }
        }
        if top_projects.len() < 3 {
            top_projects.push(result);
        }
    }
    
    let top_languages: Vec<String> = languages.iter()
        .map(|(k, v)| format!("{} ({} projects)", k, v))
        .take(3)
        .collect();
    
    let mut summary = format!(
        "## 搜索结果概览\n\n针对「{}」的搜索共找到 **{}** 个相关项目：\n\n",
        query, total_count
    );
    
    if local_count > 0 {
        summary.push_str(&format!("- **本地项目**：{} 个\n", local_count));
    }
    if web_count > 0 {
        summary.push_str(&format!("- **网络推荐**：{} 个\n", web_count));
    }
    
    if !top_languages.is_empty() {
        summary.push_str(&format!("\n### 主要语言\n{}\n", top_languages.join(" | ")));
    }
    
    if !top_projects.is_empty() {
        summary.push_str("\n### 推荐项目\n\n");
        for (i, proj) in top_projects.iter().enumerate() {
            let stars_str = proj.stars.map(|s| format!("⭐ {}", s)).unwrap_or_default();
            summary.push_str(&format!(
                "{}. **[{}]({})** {}\n",
                i + 1,
                proj.name,
                proj.url,
                stars_str
            ));
            if let Some(desc) = &proj.description {
                let desc_short = if desc.len() > 100 { format!("{}...", &desc[..100]) } else { desc.clone() };
                summary.push_str(&format!("   > {}\n", desc_short));
            }
        }
    }
    
    summary.push_str(&format!(
        "\n---\n\n💡 **提示**：点击项目名称可查看详情，或使用分类标签筛选结果。"
    ));
    
    summary
}

pub async fn analyze_intent(user_input: &str) -> Result<IntentAnalysis, String> {
    write_debug_log(&format!("[DEBUG] analyze_intent: 开始分析意图，输入: {}", user_input));

    let client = LlmClient::from_settings()
        .ok_or_else(|| {
            write_debug_log("[DEBUG] analyze_intent: LLM 未配置");
            "LLM not configured".to_string()
        })?;

    write_debug_log("[DEBUG] analyze_intent: LLM 客户端已创建");

    let prompt = build_intent_analysis_prompt(user_input);
    write_debug_log("[DEBUG] analyze_intent: Prompt 构建完成，调用 LLM...");

    let messages = vec![LlmMessage {
        role: "user".to_string(),
        content: prompt,
    }];

    let response = client.chat(messages).await
        .map_err(|e| {
            write_debug_log(&format!("[DEBUG] analyze_intent: LLM 调用失败: {}", e));
            format!("LLM request failed: {}", e)
        })?;

    write_debug_log(&format!("[DEBUG] analyze_intent: LLM 返回: {}", response));

    let parsed: IntentAnalysis = serde_json::from_str(&response)
        .map_err(|e| {
            write_debug_log(&format!("[DEBUG] analyze_intent: JSON 解析失败: {}", e));
            format!("Failed to parse LLM response: {}\nResponse: {}", e, response)
        })?;

    write_debug_log(&format!("[DEBUG] analyze_intent: 解析成功: {:?}", parsed));
    Ok(parsed)
}

fn build_intent_analysis_prompt(user_input: &str) -> String {
    format!(
        r#"你是一个开源项目推荐助手。用户输入：「{}」

请分析用户意图：
1. 明确意图：直接描述需求（如"找 Vue 状态管理库"）
2. 不明确意图：模糊描述、闲聊、无法直接搜索

如果是明确意图，返回：
{{"intent": "clear", "intent_type": "技术栈/语言/领域/场景", "keywords": ["关键词1", "关键词2"]}}

如果意图不明，返回：
{{"intent": "unclear", "questions": ["追问问题1", "追问问题2"], "options": ["选项A", "选项B"]}}

只返回JSON，不要有其他内容。"#,
        user_input
    )
}

#[async_trait]
impl FeaturePlugin for SearchPlugin {
    fn id(&self) -> &str {
        "search"
    }

    fn name(&self) -> &str {
        "意图搜索"
    }

    fn plugin_type(&self) -> FeaturePluginType {
        FeaturePluginType::Search
    }

    fn version(&self) -> &str {
        "1.0.0"
    }

    fn db_mode(&self) -> DbMode {
        DbMode::Vault
    }

    fn db_path_template(&self) -> Option<&str> {
        Some("${vault_dir}/plugin_${plugin_id}.db")
    }

    async fn init(&self, context: &PluginContext) -> Result<(), PluginError> {
        init_search_db(&context.vault_dir).map_err(|e| PluginError::InitFailed(e))?;
        Ok(())
    }

    async fn on_enable(&self, _context: &PluginContext) -> Result<(), PluginError> {
        Ok(())
    }

    async fn on_disable(&self, _context: &PluginContext) -> Result<(), PluginError> {
        Ok(())
    }

    async fn execute(&self, _context: &PluginContext) -> Result<FeatureResult, PluginError> {
        Ok(FeatureResult::success())
    }
}
