use crate::db::DATABASE;
use crate::embedding::cosine_similarity;
use crate::commands::variables::get_platform_proxy;
use crate::llm::{LlmClient, LlmMessage};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimilarProject {
    pub id: Option<i64>,
    pub name: String,
    pub url: Option<String>,
    pub description: Option<String>,
    pub source: String,
    pub language: Option<String>,
    pub stars: i64,
    pub forks: i64,
    pub score: Option<f32>,
    pub verified: bool,
    pub imported: bool,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkSimilarResult {
    pub source: String,
    pub cached_at: Option<String>,
    pub results: Vec<SimilarProject>,
    pub error: Option<String>,
}

fn parse_embedding(value: String) -> Option<Vec<f32>> {
    serde_json::from_str::<Vec<f32>>(&value).ok()
}

/// 本地项目 languages 字段存的是 JSON 数组字符串，统一转成可读文本（最多 3 个）
fn normalize_language(raw: Option<String>) -> Option<String> {
    let raw = raw?;
    let trimmed = raw.trim();
    if trimmed.is_empty() { return None; }
    if trimmed.starts_with('[') {
        if let Ok(list) = serde_json::from_str::<Vec<String>>(trimmed) {
            let joined = list.into_iter().filter(|s| !s.trim().is_empty()).take(3).collect::<Vec<_>>().join(", ");
            return if joined.is_empty() { None } else { Some(joined) };
        }
    }
    Some(trimmed.trim_matches(['\"', '\'']).to_string())
}

/// 提取有意义的检索词：长度>=3、非泛用词。skill/tool/app 等泛词单独命中不代表相似
const STOPWORDS: &[&str] = &[
    "the", "and", "for", "with", "from", "that", "this", "your", "you", "all", "are",
    "app", "tool", "tools", "project", "projects", "skill", "skills", "cli", "web", "api",
    "framework", "library", "based", "using", "use", "open", "source", "simple", "easy",
    "manager", "management", "system", "platform", "service", "server", "client",
];

fn extract_tokens(text: &str) -> Vec<String> {
    let mut seen = std::collections::HashSet::new();
    text.to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|s| s.len() >= 3 && !STOPWORDS.contains(s))
        .filter(|s| seen.insert(s.to_string()))
        .map(str::to_string)
        .collect()
}

/// 网络搜索关键词：项目名词根 + 描述前几个关键词，避免只用名字搜出一堆无关结果
fn build_search_query(name: &str, description: &str) -> String {
    let mut tokens = extract_tokens(&name.replace(['/', '_', '-'], " "));
    for token in extract_tokens(description) {
        if tokens.len() >= 5 { break; }
        if !tokens.contains(&token) { tokens.push(token); }
    }
    if tokens.is_empty() {
        return name.replace(['/', '_', '-'], " ");
    }
    tokens.join(" ")
}

const MIN_VECTOR_SCORE: f32 = 0.35;

#[tauri::command]
pub fn get_similar_projects(project_id: i64, limit: Option<usize>) -> Result<Vec<SimilarProject>, String> {
    let limit = limit.unwrap_or(10).min(50);
    let db = DATABASE.lock().map_err(|e| e.to_string())?;
    let db = db.as_ref().ok_or("Database not initialized")?;
    let conn = db.get_connection();
    let query_embedding: Option<String> = conn.query_row(
        "SELECT embedding FROM project_embeddings WHERE project_id = ?",
        [project_id],
        |row| row.get(0),
    ).ok();
    let Some(query_embedding) = query_embedding.and_then(parse_embedding) else {
        return Ok(rule_based_similar(&conn, project_id, limit));
    };
    let mut stmt = conn.prepare(
        "SELECT p.id, p.name, p.url, COALESCE(NULLIF(p.ai_summary,''), p.description), p.source, p.languages, p.stars, p.forks, e.embedding
         FROM projects p JOIN project_embeddings e ON e.project_id = p.id
         WHERE p.id != ? AND p.data_status = 'ACTIVE'"
    ).map_err(|e| e.to_string())?;
    let mut scored = Vec::new();
    let rows = stmt.query_map([project_id], |row| {
        Ok((
            row.get::<_, i64>(0)?, row.get::<_, String>(1)?, row.get::<_, Option<String>>(2)?,
            row.get::<_, Option<String>>(3)?, row.get::<_, String>(4)?, row.get::<_, Option<String>>(5)?,
            row.get::<_, i64>(6)?, row.get::<_, i64>(7)?, row.get::<_, String>(8)?,
        ))
    }).map_err(|e| e.to_string())?;
    for row in rows.flatten() {
        let (id, name, url, description, source, language, stars, forks, embedding) = row;
        if let Some(vector) = parse_embedding(embedding) {
            if vector.len() == query_embedding.len() {
                let score = cosine_similarity(&query_embedding, &vector);
                if score < MIN_VECTOR_SCORE { continue; }
                scored.push(SimilarProject { id: Some(id), name, url, description, source, language: normalize_language(language),
                    stars, forks, score: Some(score), verified: true,
                    imported: true, reason: Some("vector".to_string()) });
            }
        }
    }
    scored.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
    scored.truncate(limit);
    Ok(scored)
}

fn rule_based_similar(conn: &rusqlite::Connection, project_id: i64, limit: usize) -> Vec<SimilarProject> {
    let current: (String, String, Option<String>, Option<i64>) = match conn.query_row(
        "SELECT name, COALESCE(description,''), languages, category_id FROM projects WHERE id = ?", [project_id],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
    ) { Ok(value) => value, Err(_) => return Vec::new() };
    let current_language = normalize_language(current.2.clone()).map(|s| s.to_lowercase());
    // 关键词来源：名称 + 描述前 200 字符；skill/tool/app 等泛词已在 extract_tokens 中剔除
    let desc_head: String = current.1.chars().take(200).collect();
    let query_tokens: std::collections::HashSet<String> = extract_tokens(&format!("{} {}", current.0.replace(['/', '_', '-'], " "), desc_head)).into_iter().collect();
    let mut stmt = match conn.prepare("SELECT id, name, url, COALESCE(NULLIF(ai_summary,''), description), source, languages, stars, forks, category_id FROM projects WHERE id != ? AND data_status = 'ACTIVE'") { Ok(stmt) => stmt, Err(_) => return Vec::new() };
    let rows = match stmt.query_map([project_id], |row| Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?, row.get::<_, Option<String>>(2)?, row.get::<_, Option<String>>(3)?, row.get::<_, String>(4)?, row.get::<_, Option<String>>(5)?, row.get::<_, i64>(6)?, row.get::<_, i64>(7)?, row.get::<_, Option<i64>>(8)?))) { Ok(rows) => rows, Err(_) => return Vec::new() };
    let mut results = rows.flatten().filter_map(|(id, name, url, description, source, languages, stars, forks, category_id)| {
        let language = normalize_language(languages);
        let same_language = current_language.as_deref().zip(language.as_deref()).map(|(a, b)| a == b.to_lowercase()).unwrap_or(false);
        let same_category = current.3.is_some() && current.3 == category_id;
        let candidate_text = format!("{} {}", name.replace(['/', '_', '-'], " "), description.as_deref().unwrap_or(""));
        let overlap = extract_tokens(&candidate_text).iter().filter(|token| query_tokens.contains(*token)).count();
        // 关键词零命中一律不推荐；仅 1 个命中时需语言+分类双重佐证，杜绝"同语言/含 skill 即相似"的噪音
        if overlap == 0 || (overlap == 1 && !(same_language && same_category)) { return None; }
        let score = (overlap.min(4) as f32 * 0.2) + (if same_language { 0.1 } else { 0.0 }) + (if same_category { 0.15 } else { 0.0 });
        Some(SimilarProject { id: Some(id), name, url, description, source, language, stars, forks, score: Some(score.min(0.99)), verified: true, imported: true, reason: Some("rule".to_string()) })
    }).collect::<Vec<_>>();
    results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
    results.truncate(limit);
    results
}

fn project_info(project_id: i64) -> Result<(String, String, String, Option<String>, Option<String>), String> {
    let db = DATABASE.lock().map_err(|e| e.to_string())?;
    let db = db.as_ref().ok_or("Database not initialized")?;
    let conn = db.get_connection();
    conn.query_row(
        "SELECT name, source, COALESCE(description,''), languages, url FROM projects WHERE id = ? AND data_status = 'ACTIVE'",
        [project_id], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?))
    ).map_err(|e| e.to_string())
}

fn cached(conn: &rusqlite::Connection, project_id: i64, channel: &str, refresh: bool) -> Option<NetworkSimilarResult> {
    if refresh { return None; }
    let row: (String, String) = conn.query_row(
        "SELECT results_json, created_at FROM similar_projects_cache WHERE project_id = ? AND channel = ? AND datetime(created_at) >= datetime('now', '-7 days')",
        rusqlite::params![project_id, channel], |r| Ok((r.get(0)?, r.get(1)?))
    ).ok()?;
    let results = serde_json::from_str(&row.0).ok()?;
    Some(NetworkSimilarResult { source: channel.to_string(), cached_at: Some(row.1), results, error: None })
}

fn value_string(value: &Value, key: &str) -> Option<String> { value.get(key).and_then(Value::as_str).map(str::to_string) }
fn value_i64(value: &Value, key: &str) -> i64 { value.get(key).and_then(Value::as_i64).unwrap_or(0) }

async fn search_platform(name: &str, source: &str, description: &str, language: Option<&str>, proxy: Option<String>) -> Result<Vec<SimilarProject>, String> {
    let query = build_search_query(name, description);
    let language = normalize_language(language.map(str::to_string));
    let client = {
        let mut builder = reqwest::Client::builder().timeout(std::time::Duration::from_secs(30));
        if let Some(proxy) = proxy.filter(|p| !p.trim().is_empty()) { builder = builder.proxy(reqwest::Proxy::all(proxy).map_err(|e| e.to_string())?); }
        builder.build().map_err(|e| e.to_string())?
    };
    let github_q = match language.as_deref() {
        Some(lang) if !lang.is_empty() => format!("{} language:{}", query, lang.split(',').next().unwrap_or("").trim()),
        _ => query.clone(),
    };
    let (url, source_name) = match source {
        "github" => (format!("https://api.github.com/search/repositories?q={}&sort=stars&per_page=10", urlencoding::encode(&github_q)), "github"),
        "gitee" => (format!("https://gitee.com/api/v5/search/repositories?q={}&page=1&per_page=10", urlencoding::encode(&query)), "gitee"),
        "gitlab" => (format!("https://gitlab.com/api/v4/projects?search={}&per_page=10&order_by=star_count", urlencoding::encode(&query)), "gitlab"),
        "npm" => (format!("https://registry.npmjs.org/-/v1/search?text={}&size=10", urlencoding::encode(&query)), "npm"),
        "crates" => (format!("https://crates.io/api/v1/crates?q={}&per_page=10", urlencoding::encode(&query)), "crates"),
        _ => return Ok(Vec::new()),
    };
    let mut request = client.get(url).header("User-Agent", "OS-Compass");
    if source_name == "github" { request = request.header("Accept", "application/vnd.github+json"); }
    let response = request.send().await.map_err(|e| format!("网络请求失败: {}", e))?;
    if !response.status().is_success() { return Err(format!("{} API 返回 {}", source_name, response.status())); }
    let body: Value = response.json().await.map_err(|e| format!("解析响应失败: {}", e))?;
    let items = body.get("items").or_else(|| body.get("crates")).or_else(|| body.get("objects")).and_then(Value::as_array).cloned().unwrap_or_default();
    Ok(items.into_iter().filter_map(|item| {
        let repo = item.get("repository").unwrap_or(&item);
        let name = value_string(&item, "full_name").or_else(|| value_string(&item, "name")).or_else(|| value_string(repo, "full_name"))?;
        let url = value_string(&item, "html_url").or_else(|| value_string(&item, "web_url")).or_else(|| value_string(&item, "homepage"));
        Some(SimilarProject { id: None, name, url, description: value_string(&item, "description").or_else(|| value_string(repo, "description")), source: source_name.to_string(), language: normalize_language(value_string(&item, "language")), stars: value_i64(&item, "stargazers_count").max(value_i64(&item, "stars")).max(value_i64(&item, "star_count")), forks: value_i64(&item, "forks_count"), score: None, verified: true, imported: false, reason: Some("platform_search".to_string()) })
    }).collect())
}

#[derive(Debug, Deserialize)]
struct LlmSimilarProject {
    name: String,
    url: Option<String>,
    description: Option<String>,
    source: Option<String>,
    language: Option<String>,
    stars: Option<i64>,
    forks: Option<i64>,
    reason: Option<String>,
}

fn verify_api_url(url: &str) -> Option<String> {
    let trimmed = url.trim().trim_end_matches('/');
    let rest = if let Some(rest) = trimmed.strip_prefix("https://github.com/").or_else(|| trimmed.strip_prefix("http://github.com/")) {
        format!("https://api.github.com/repos/{}", rest)
    } else if let Some(rest) = trimmed.strip_prefix("https://gitee.com/").or_else(|| trimmed.strip_prefix("http://gitee.com/")) {
        format!("https://gitee.com/api/v5/repos/{}", rest)
    } else {
        return None;
    };
    let parts: Vec<&str> = rest.rsplit('/').take(2).collect();
    if parts.len() == 2 && parts.iter().all(|part| !part.is_empty() && !part.contains('?')) {
        Some(rest)
    } else {
        None
    }
}

fn build_proxy_client(proxy: Option<String>) -> Option<reqwest::Client> {
    let mut builder = reqwest::Client::builder().timeout(std::time::Duration::from_secs(20));
    if let Some(proxy) = proxy.filter(|p| !p.trim().is_empty()) {
        builder = builder.proxy(reqwest::Proxy::all(proxy).ok()?);
    }
    builder.build().ok()
}

async fn verify_llm_projects(mut projects: Vec<SimilarProject>) -> Vec<SimilarProject> {
    let github_client = build_proxy_client(get_platform_proxy("github_proxy"));
    let gitee_client = build_proxy_client(get_platform_proxy("network_proxy"));
    for project in projects.iter_mut() {
        let Some(url) = project.url.as_deref().and_then(verify_api_url) else {
            project.verified = false;
            continue;
        };
        let client = if url.contains("api.github.com") { github_client.as_ref() } else { gitee_client.as_ref() };
        let Some(client) = client else {
            project.verified = false;
            continue;
        };
        let mut request = client.get(&url).header("User-Agent", "OS-Compass");
        if url.contains("api.github.com") { request = request.header("Accept", "application/vnd.github+json"); }
        match request.send().await {
            Ok(response) if response.status().is_success() => {
                if let Ok(body) = response.json::<Value>().await {
                    project.verified = true;
                    project.stars = value_i64(&body, "stargazers_count").max(value_i64(&body, "stars")).max(project.stars);
                    project.forks = value_i64(&body, "forks_count").max(project.forks);
                    if project.description.is_none() { project.description = value_string(&body, "description"); }
                }
            }
            _ => project.verified = false,
        }
    }
    projects
}

fn parse_llm_projects(content: &str) -> Result<Vec<SimilarProject>, String> {
    let cleaned = content.trim().trim_start_matches("```json").trim_start_matches("```").trim_end_matches("```").trim();
    let raw: Vec<LlmSimilarProject> = serde_json::from_str(cleaned).map_err(|e| format!("LLM 返回的相似项目不是有效 JSON: {}", e))?;
    Ok(raw.into_iter().take(10).map(|item| {
        SimilarProject {
            id: None,
            name: item.name,
            url: item.url,
            description: item.description,
            source: item.source.unwrap_or_else(|| "other".to_string()),
            language: item.language,
            stars: item.stars.unwrap_or(0),
            forks: item.forks.unwrap_or(0),
            score: None,
            verified: false,
            imported: false,
            reason: item.reason.or_else(|| Some("llm_fallback".to_string())),
        }
    }).collect())
}

#[tauri::command]
pub async fn llm_similar_projects(project_id: i64) -> Result<Vec<SimilarProject>, String> {
    let (name, source, description, language, _) = project_info(project_id)?;
    let Some(client) = LlmClient::from_settings() else {
        return Err("LLM 尚未配置，无法执行智能兜底搜索".to_string());
    };
    let prompt = format!(
        "Find up to 10 genuinely similar open-source projects to this project. Match by FUNCTIONALITY and DOMAIN (what the project does and who it serves), never by programming language alone. Prefer GitHub or Gitee URLs and do not invent URLs. Return ONLY a JSON array; each item must contain name, url, description, source, language, stars, forks, reason. Project name: {}\nSource: {}\nDescription: {}\nLanguage: {}",
        name, source, description, language.unwrap_or_default()
    );
    let content = client.chat(vec![
        LlmMessage { role: "system".to_string(), content: "You recommend open-source projects. Return strict JSON only.".to_string() },
        LlmMessage { role: "user".to_string(), content: prompt },
    ]).await?;
    let projects = parse_llm_projects(&content)?;
    Ok(verify_llm_projects(projects).await)
}

#[tauri::command]
pub async fn get_network_similar_projects(project_id: i64, refresh: Option<bool>) -> Result<NetworkSimilarResult, String> {
    let refresh = refresh.unwrap_or(false);
    let (name, source, description, language, _) = project_info(project_id)?;
    let channel = source.clone();
    let cached_result = {
        let db = DATABASE.lock().map_err(|e| e.to_string())?;
        let db = db.as_ref().ok_or("Database not initialized")?;
        let connection = db.get_connection();
        cached(&connection, project_id, &channel, refresh)
    };
    if let Some(result) = cached_result { return Ok(result); }
    let proxy_key = if source == "github" { "github_proxy" } else { "network_proxy" };
    let result = search_platform(&name, &source, &description, language.as_deref(), get_platform_proxy(proxy_key)).await;
    let results = result?;
    let json = serde_json::to_string(&results).map_err(|e| e.to_string())?;
    let db = DATABASE.lock().map_err(|e| e.to_string())?;
    let db = db.as_ref().ok_or("Database not initialized")?;
    let conn = db.get_connection();
    conn.execute("INSERT OR REPLACE INTO similar_projects_cache (project_id, channel, results_json, created_at) VALUES (?, ?, ?, datetime('now','localtime'))", rusqlite::params![project_id, channel, json]).map_err(|e| e.to_string())?;
    Ok(NetworkSimilarResult { source, cached_at: None, results, error: None })
}
