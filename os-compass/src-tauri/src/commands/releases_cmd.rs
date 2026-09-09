use crate::db::DATABASE;
use crate::translate;
use chrono::Local;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadUrl {
    pub name: String,
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Release {
    pub id: String,
    pub project_id: i64,
    pub tag_name: String,
    pub published_at: Option<String>,
    pub body: Option<String>,
    pub body_zh: Option<String>,
    pub download_urls: Vec<DownloadUrl>,
    pub source: String,
    pub fetched_at: Option<String>,
}

fn parse_download_urls(json: Option<&str>) -> Vec<DownloadUrl> {
    match json {
        Some(s) if !s.is_empty() => serde_json::from_str(s).unwrap_or_default(),
        _ => Vec::new(),
    }
}

#[tauri::command]
pub fn get_releases(project_id: i64) -> Result<Vec<Release>, String> {
    let db_guard = DATABASE.try_lock().map_err(|e| e.to_string())?;
    let db = db_guard.as_ref().ok_or("Database not initialized")?;
    let conn = db.get_connection();

    let mut stmt = conn.prepare(
        "SELECT id, project_id, tag_name, published_at, body, body_zh, download_urls, source, fetched_at 
         FROM project_releases WHERE project_id = ? ORDER BY published_at DESC"
    ).map_err(|e| e.to_string())?;

    let releases = stmt.query_map([project_id], |row| {
        Ok(Release {
            id: row.get(0)?,
            project_id: row.get(1)?,
            tag_name: row.get(2)?,
            published_at: row.get(3)?,
            body: row.get(4)?,
            body_zh: row.get(5)?,
            download_urls: parse_download_urls(row.get::<_, Option<String>>(6)?.as_deref()),
            source: row.get(7)?,
            fetched_at: row.get(8)?,
        })
    }).map_err(|e| e.to_string())?;

    let mut result = Vec::new();
    for release in releases {
        result.push(release.map_err(|e| e.to_string())?);
    }
    Ok(result)
}

#[derive(Debug, Deserialize)]
struct GitHubRelease {
    id: i64,
    tag_name: String,
    published_at: Option<String>,
    body: Option<String>,
    assets: Vec<GitHubAsset>,
}

#[derive(Debug, Deserialize)]
struct GitHubAsset {
    name: String,
    browser_download_url: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GiteeRelease {
    id: i64,
    tag_name: String,
    #[serde(rename = "published_at")]
    published_at: Option<String>,
    body: Option<String>,
}

fn fetch_github_releases(url: &str, token: Option<&str>, proxy: Option<&str>) -> Result<Vec<Release>, String> {
    let owner_repo = url
        .trim_end_matches('/')
        .trim_end_matches("/releases");

    let parts: Vec<&str> = owner_repo.rsplit('/').take(2).collect();
    if parts.len() < 2 {
        return Err("Invalid GitHub URL".to_string());
    }
    let owner = parts[1];
    let repo = parts[0];

    let api_url = format!("https://api.github.com/repos/{}/{}/releases", owner, repo);

    let runtime = tokio::runtime::Runtime::new().map_err(|e| e.to_string())?;
    let result = runtime.block_on(async {
        let mut client_builder = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30));
        if let Some(p) = proxy {
            if !p.trim().is_empty() {
                if let Ok(proxy_obj) = reqwest::Proxy::all(p.trim()) {
                    client_builder = client_builder.proxy(proxy_obj);
                }
            }
        }
        let client = client_builder.build().map_err(|e| e.to_string())?;

        let mut req = client
            .get(&api_url)
            .header("User-Agent", "OS-Compass")
            .header("Accept", "application/vnd.github.v3+json");

        if let Some(t) = token {
            req = req.header("Authorization", format!("Bearer {}", t));
        }

        let resp = req.send().await.map_err(|e| e.to_string())?;

        if resp.status() == 403 {
            return Err("API_RATE_LIMITED".to_string());
        }

        if !resp.status().is_success() {
            return Err(format!("GitHub API error: {}", resp.status()));
        }

        let gh_releases: Vec<GitHubRelease> = resp.json().await.map_err(|e| e.to_string())?;

        Ok(gh_releases.into_iter().map(|r| {
            let download_urls: Vec<DownloadUrl> = r.assets
                .into_iter()
                .filter_map(|a| a.browser_download_url.map(|url| DownloadUrl { name: a.name, url }))
                .collect();

            Release {
                id: r.id.to_string(),
                project_id: 0,
                tag_name: r.tag_name,
                published_at: r.published_at,
                body: r.body,
                body_zh: None,
                download_urls,
                source: "github".to_string(),
                fetched_at: Some(Local::now().format("%Y-%m-%d %H:%M:%S").to_string()),
            }
        }).collect::<Vec<_>>())
    });

    result
}

fn fetch_gitee_releases(url: &str, token: Option<&str>) -> Result<Vec<Release>, String> {
    let owner_repo = url
        .trim_end_matches('/')
        .trim_end_matches("/releases");

    let parts: Vec<&str> = owner_repo.rsplit('/').take(2).collect();
    if parts.len() < 2 {
        return Err("Invalid Gitee URL".to_string());
    }
    let owner = parts[1];
    let repo = parts[0];

    let api_url = format!("https://gitee.com/api/v5/repos/{}/{}/releases", owner, repo);

    let runtime = tokio::runtime::Runtime::new().map_err(|e| e.to_string())?;
    let result = runtime.block_on(async {
        let mut req = reqwest::Client::new()
            .get(&api_url);

        if let Some(t) = token {
            req = req.query(&[("access_token", t)]);
        }

        let resp = req.send().await.map_err(|e| e.to_string())?;

        if !resp.status().is_success() {
            return Err(format!("Gitee API error: {}", resp.status()));
        }

        let gitee_releases: Vec<GiteeRelease> = resp.json().await.map_err(|e| e.to_string())?;

        Ok(gitee_releases.into_iter().map(|r| {
            Release {
                id: r.id.to_string(),
                project_id: 0,
                tag_name: r.tag_name,
                published_at: r.published_at,
                body: r.body,
                body_zh: None,
                download_urls: Vec::new(),
                source: "gitee".to_string(),
                fetched_at: Some(Local::now().format("%Y-%m-%d %H:%M:%S").to_string()),
            }
        }).collect::<Vec<_>>())
    });

    result
}

#[tauri::command]
pub fn fetch_releases(project_id: i64) -> Result<Vec<Release>, String> {
    let db_guard = DATABASE.try_lock().map_err(|e| e.to_string())?;
    let db = db_guard.as_ref().ok_or("Database not initialized")?;
    let conn = db.get_connection();

    let project_url: Option<String> = conn.query_row(
        "SELECT url FROM projects WHERE id = ?",
        [project_id],
        |row| row.get(0),
    ).ok();

    let source: Option<String> = conn.query_row(
        "SELECT source FROM projects WHERE id = ?",
        [project_id],
        |row| row.get(0),
    ).ok();

    drop(conn);
    drop(db_guard);

    let url = project_url.ok_or("NO_URL")?;
    let platform = source.unwrap_or_default();

    let releases = match platform.as_str() {
        "github" => {
            let token = crate::commands::variables::get_variable_value_internal("github_token");
            let proxy = crate::commands::variables::get_variable_value_internal("github_proxy");
            fetch_github_releases(&url, token.as_deref(), proxy.as_deref())?
        },
        "gitee" => {
            let token = crate::commands::variables::get_variable_value_internal("gitee_token");
            fetch_gitee_releases(&url, token.as_deref())?
        },
        _ => return Err("PLATFORM_NOT_SUPPORTED".to_string()),
    };

    let db_guard = DATABASE.try_lock().map_err(|e| e.to_string())?;
    let db = db_guard.as_ref().ok_or("Database not initialized")?;
    let conn = db.get_connection();

    conn.execute("DELETE FROM project_releases WHERE project_id = ?", [project_id])
        .map_err(|e| e.to_string())?;

    for mut release in releases {
        release.project_id = project_id;
        let download_urls_json = serde_json::to_string(&release.download_urls).unwrap_or_default();

        conn.execute(
            "INSERT INTO project_releases (id, project_id, tag_name, published_at, body, body_zh, download_urls, source, fetched_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
            rusqlite::params![
                release.id,
                release.project_id,
                release.tag_name,
                release.published_at,
                release.body,
                release.body_zh,
                download_urls_json,
                release.source,
                release.fetched_at,
            ],
        ).map_err(|e| e.to_string())?;
    }

    drop(conn);
    drop(db_guard);

    get_releases(project_id)
}

#[tauri::command]
pub async fn translate_release_body(project_id: String, release_id: String, text: String) -> Result<String, String> {
    let result = translate::translate_text(&text, "zh-CN").await?;

    let db_guard = DATABASE.try_lock().map_err(|e| e.to_string())?;
    let db = db_guard.as_ref().ok_or("Database not initialized")?;
    let conn = db.get_connection();

    conn.execute(
        "UPDATE project_releases SET body_zh = ? WHERE id = ? AND project_id = ?",
        rusqlite::params![result.clone(), release_id, project_id],
    ).map_err(|e| e.to_string())?;

    Ok(result)
}
