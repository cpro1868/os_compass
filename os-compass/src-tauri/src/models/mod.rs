use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Project {
    pub id: i64,
    pub name: String,
    pub url: Option<String>,
    pub source: String,
    pub source_id: Option<String>,
    pub description: Option<String>,
    pub languages: Option<String>,
    pub stars: i32,
    pub forks: i32,
    pub open_issues: i32,
    pub license: Option<String>,
    pub homepage: Option<String>,
    pub latest_commit: Option<String>,
    pub latest_release: Option<String>,
    pub readme_content: Option<String>,
    pub readme_lang: Option<String>,
    pub health_score: Option<f64>,
    pub ai_summary: Option<String>,
    pub ai_use_cases: Option<String>,
    pub ai_risks: Option<String>,
    pub ai_dependencies: Option<String>,
    pub translated_summary: Option<String>,
    pub translated_use_cases: Option<String>,
    pub translated_risks: Option<String>,
    pub translated_dependencies: Option<String>,
    pub translated_description: Option<String>,
    pub readme_translation: Option<String>,
    pub category_id: Option<i64>,
    pub lifecycle_status: String,
    pub data_status: String,
    pub is_downloaded: bool,
    pub local_path: Option<String>,
    pub runbook: Option<String>,
    pub archived_at: Option<String>,
    pub deleted_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReadmeVariant {
    pub id: i64,
    pub project_id: i64,
    pub file_name: String,
    pub language: String,
    pub content: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Translation {
    pub id: i64,
    pub project_id: i64,
    pub field_name: String,
    pub language: String,
    pub content: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Category {
    pub id: i64,
    pub name: String,
    pub path: String,
    pub explain: Option<String>,
    pub sort_order: i32,
    pub parent_id: Option<i64>,
    pub is_system: bool,
    pub project_count: i64,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tag {
    pub id: i64,
    pub name: String,
    pub color: Option<String>,
    pub source: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectNote {
    pub id: i64,
    pub project_id: i64,
    pub content: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectUserInfo {
    pub id: i64,
    pub project_id: i64,
    pub info_key: String,
    pub info_value: Option<String>,
    pub is_secret: bool,
    pub created_at: String,
    pub updated_at: String,
}
