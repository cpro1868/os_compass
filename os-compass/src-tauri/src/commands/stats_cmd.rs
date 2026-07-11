use crate::db::DATABASE;
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct StatsResponse {
    pub total_projects: i64,
    pub status_distribution: Vec<StatusCount>,
    pub health_distribution: Vec<HealthBucket>,
    pub category_distribution: Vec<CategoryCount>,
    pub language_distribution: Vec<LanguageCount>,
    pub tag_distribution: Vec<TagCount>,
}

#[derive(Debug, Serialize)]
pub struct StatusCount {
    pub status: String,
    pub count: i64,
}

#[derive(Debug, Serialize)]
pub struct HealthBucket {
    pub label: String,
    pub min: f64,
    pub max: f64,
    pub count: i64,
}

#[derive(Debug, Serialize)]
pub struct CategoryCount {
    pub category_id: Option<i64>,
    pub category_name: String,
    pub count: i64,
}

#[derive(Debug, Serialize)]
pub struct LanguageCount {
    pub language: String,
    pub count: i64,
}

#[derive(Debug, Serialize)]
pub struct TagCount {
    pub tag_id: i64,
    pub tag_name: String,
    pub count: i64,
}

#[tauri::command]
pub fn get_stats() -> Result<StatsResponse, String> {
    let db = DATABASE.lock().map_err(|e| e.to_string())?;
    let db = db.as_ref().ok_or("Database not initialized")?;
    let conn = db.get_connection();

    let total_projects: i64 = conn
        .query_row("SELECT COUNT(*) FROM projects WHERE data_status = 'ACTIVE'", [], |row| row.get(0))
        .unwrap_or(0);

    let mut stmt = conn
        .prepare("SELECT lifecycle_status, COUNT(*) FROM projects WHERE data_status = 'ACTIVE' GROUP BY lifecycle_status")
        .map_err(|e| e.to_string())?;
    let status_distribution = stmt
        .query_map([], |row| Ok(StatusCount {
            status: row.get(0)?,
            count: row.get(1)?,
        }))
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    let buckets = [
        ("非常健康 (80-100)", 80.0, 100.0),
        ("健康 (60-79)", 60.0, 79.99),
        ("一般 (40-59)", 40.0, 59.99),
        ("不健康 (0-39)", 0.0, 39.99),
    ];
    let mut health_distribution = Vec::new();
    for (label, min, max) in &buckets {
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM projects WHERE data_status = 'ACTIVE' AND health_score IS NOT NULL AND health_score >= ? AND health_score <= ?",
                rusqlite::params![min, max],
                |row| row.get(0),
            )
            .unwrap_or(0);
        if count > 0 {
            health_distribution.push(HealthBucket {
                label: label.to_string(),
                min: *min,
                max: *max,
                count,
            });
        }
    }

    let mut stmt = conn
        .prepare(
            "SELECT p.category_id, COALESCE(c.name, '未分类'), COUNT(*) FROM projects p LEFT JOIN categories c ON p.category_id = c.id WHERE p.data_status = 'ACTIVE' GROUP BY p.category_id ORDER BY COUNT(*) DESC",
        )
        .map_err(|e| e.to_string())?;
    let category_distribution = stmt
        .query_map([], |row| Ok(CategoryCount {
            category_id: row.get(0)?,
            category_name: row.get(1)?,
            count: row.get(2)?,
        }))
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    let mut lang_map: std::collections::HashMap<String, i64> = std::collections::HashMap::new();
    let mut stmt = conn
        .prepare("SELECT languages FROM projects WHERE data_status = 'ACTIVE' AND languages IS NOT NULL")
        .map_err(|e| e.to_string())?;
    let lang_rows = stmt
        .query_map([], |row| row.get::<_, String>(0))
        .map_err(|e| e.to_string())?;
    for row in lang_rows {
        if let Ok(langs_json) = row {
            if let Ok(arr) = serde_json::from_str::<Vec<String>>(&langs_json) {
                for l in arr {
                    *lang_map.entry(l).or_insert(0) += 1;
                }
            } else {
                *lang_map.entry(langs_json).or_insert(0) += 1;
            }
        }
    }
    let mut language_distribution: Vec<LanguageCount> = lang_map
        .into_iter()
        .map(|(language, count)| LanguageCount { language, count })
        .collect();
    language_distribution.sort_by(|a, b| b.count.cmp(&a.count));
    language_distribution.truncate(15);

    let mut stmt = conn
        .prepare(
            "SELECT t.id, t.name, COUNT(pt.project_id) as cnt FROM tags t INNER JOIN project_tags pt ON t.id = pt.tag_id INNER JOIN projects p ON pt.project_id = p.id WHERE p.data_status = 'ACTIVE' GROUP BY t.id ORDER BY cnt DESC LIMIT 20",
        )
        .map_err(|e| e.to_string())?;
    let tag_distribution = stmt
        .query_map([], |row| Ok(TagCount {
            tag_id: row.get(0)?,
            tag_name: row.get(1)?,
            count: row.get(2)?,
        }))
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    Ok(StatsResponse {
        total_projects,
        status_distribution,
        health_distribution,
        category_distribution,
        language_distribution,
        tag_distribution,
    })
}
