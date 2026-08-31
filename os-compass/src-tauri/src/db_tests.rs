#[cfg(test)]
mod tests {
    use rusqlite::{Connection, Result};
    use std::path::PathBuf;
    use tempfile::tempdir;

    fn create_test_db() -> Result<Connection> {
        let dir = tempdir().unwrap();
        let db_path: PathBuf = dir.path().join("test.db");
        let conn = Connection::open(&db_path)?;

        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS projects (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL,
                url TEXT,
                source TEXT NOT NULL,
                source_id TEXT,
                description TEXT,
                languages TEXT,
                stars INTEGER DEFAULT 0,
                forks INTEGER DEFAULT 0,
                open_issues INTEGER DEFAULT 0,
                license TEXT,
                homepage TEXT,
                latest_commit TEXT,
                latest_release TEXT,
                readme_content TEXT,
                readme_lang TEXT DEFAULT 'en',
                health_score REAL,
                ai_summary TEXT,
                ai_use_cases TEXT,
                ai_risks TEXT,
                ai_dependencies TEXT,
                category_id INTEGER,
                lifecycle_status TEXT DEFAULT 'TO_EXPLORE',
                data_status TEXT DEFAULT 'ACTIVE',
                is_downloaded INTEGER DEFAULT 0,
                local_path TEXT,
                archived_at TEXT,
                deleted_at TEXT,
                created_at TEXT DEFAULT (datetime('now', 'localtime')),
                updated_at TEXT DEFAULT (datetime('now', 'localtime'))
            );
            "#,
        )?;

        Ok(conn)
    }

    #[test]
    fn test_get_projects_query_returns_all_columns() {
        let conn = create_test_db().unwrap();

        conn.execute(
            "INSERT INTO projects (name, url, source, description, stars) VALUES (?, ?, ?, ?, ?)",
            rusqlite::params!["test-project", "https://github.com/test/project", "github", "A test project", 100],
        )
        .unwrap();

        let mut stmt = conn
            .prepare(
                "SELECT id, name, url, source, source_id, description, languages, stars, forks, open_issues, license, homepage, latest_commit, latest_release, readme_content, readme_lang, health_score, ai_summary, ai_use_cases, ai_risks, ai_dependencies, category_id, lifecycle_status, data_status, is_downloaded, local_path, archived_at, deleted_at, created_at, updated_at FROM projects WHERE data_status = 'ACTIVE'",
            )
            .unwrap();

        let projects: Vec<(i64, String, Option<String>, String, Option<String>, Option<String>, Option<String>, i32, i32, i32, Option<String>, Option<String>, Option<String>, Option<String>, Option<String>, Option<String>, Option<f64>, Option<String>, Option<String>, Option<String>, Option<String>, Option<i64>, String, String, i32, Option<String>, Option<String>, Option<String>, String, String)> = stmt
            .query_map([], |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                    row.get(5)?,
                    row.get(6)?,
                    row.get(7)?,
                    row.get(8)?,
                    row.get(9)?,
                    row.get(10)?,
                    row.get(11)?,
                    row.get(12)?,
                    row.get(13)?,
                    row.get(14)?,
                    row.get(15)?,
                    row.get(16)?,
                    row.get(17)?,
                    row.get(18)?,
                    row.get(19)?,
                    row.get(20)?,
                    row.get(21)?,
                    row.get(22)?,
                    row.get(23)?,
                    row.get::<_, i32>(24)?,
                    row.get(25)?,
                    row.get(26)?,
                    row.get(27)?,
                    row.get(28)?,
                    row.get(29)?,
                ))
            })
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();

        assert_eq!(projects.len(), 1);
        let p = &projects[0];
        assert_eq!(p.1, "test-project");
        assert_eq!(p.3, "github");
        assert_eq!(p.7, 100);
        assert_eq!(p.22, "TO_EXPLORE");
        assert_eq!(p.23, "ACTIVE");
        assert_eq!(p.24, 0);
    }

    #[test]
    fn test_insert_and_query_project() {
        let conn = create_test_db().unwrap();

        let id: i64 = conn
            .execute(
                "INSERT INTO projects (name, url, source, description, stars, forks, open_issues, license, homepage, latest_commit, latest_release, readme_content, readme_lang, health_score, ai_summary, ai_use_cases, ai_risks, ai_dependencies, category_id, lifecycle_status, data_status, is_downloaded, local_path, archived_at, deleted_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
                rusqlite::params![
                    "rust-lang/rust",
                    "https://github.com/rust-lang/rust",
                    "github",
                    "A language empowering everyone to build reliable and efficient software.",
                    100000,
                    25000,
                    5000,
                    "MIT",
                    "https://www.rust-lang.org",
                    "2024-01-01",
                    "1.75.0",
                    "# Rust",
                    "en",
                    95.5,
                    "Summary",
                    "Use cases",
                    "Risks",
                    "Dependencies",
                    1,
                    "TO_EXPLORE",
                    "ACTIVE",
                    0,
                    None::<String>,
                    None::<String>,
                    None::<String>,
                ],
            )
            .unwrap() as i64;

        assert!(id > 0);

        let mut stmt = conn
            .prepare("SELECT id, name, stars, health_score FROM projects WHERE id = ?")
            .unwrap();
        let row: (i64, String, i32, Option<f64>) = stmt
            .query_row([id], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)))
            .unwrap();

        assert_eq!(row.0, id);
        assert_eq!(row.1, "rust-lang/rust");
        assert_eq!(row.2, 100000);
        assert_eq!(row.3, Some(95.5));
    }

    #[test]
    fn test_archived_projects_query() {
        let conn = create_test_db().unwrap();

        conn.execute(
            "INSERT INTO projects (name, url, source, data_status, archived_at) VALUES (?, ?, ?, ?, datetime('now', 'localtime'))",
            rusqlite::params!["archived-project", "https://github.com/test/archived", "github", "ARCHIVED"],
        )
        .unwrap();

        conn.execute(
            "INSERT INTO projects (name, url, source, data_status) VALUES (?, ?, ?, ?)",
            rusqlite::params!["active-project", "https://github.com/test/active", "github", "ACTIVE"],
        )
        .unwrap();

        let mut stmt = conn
            .prepare("SELECT id, name, data_status, archived_at FROM projects WHERE data_status = 'ARCHIVED'")
            .unwrap();
        let archived: Vec<(i64, String, String, Option<String>)> = stmt
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)))
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();

        assert_eq!(archived.len(), 1);
        assert_eq!(archived[0].1, "archived-project");
        assert!(archived[0].3.is_some());

        let mut stmt = conn
            .prepare("SELECT id, name FROM projects WHERE data_status = 'ACTIVE'")
            .unwrap();
        let active: Vec<(i64, String)> = stmt
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();

        assert_eq!(active.len(), 1);
        assert_eq!(active[0].1, "active-project");
    }

    #[test]
    fn test_update_project_status() {
        let conn = create_test_db().unwrap();

        conn.execute(
            "INSERT INTO projects (name, url, source, lifecycle_status) VALUES (?, ?, ?, ?)",
            rusqlite::params!["test", "https://test.com", "github", "TO_EXPLORE"],
        )
        .unwrap();

        let updated = conn
            .execute(
                "UPDATE projects SET lifecycle_status = 'DIVING', updated_at = datetime('now', 'localtime') WHERE name = 'test'",
                [],
            )
            .unwrap();

        assert_eq!(updated, 1);

        let mut stmt = conn
            .prepare("SELECT lifecycle_status FROM projects WHERE name = 'test'")
            .unwrap();
        let status: String = stmt
            .query_row([], |row| row.get(0))
            .unwrap();

        assert_eq!(status, "DIVING");
    }

    fn create_test_db_with_categories() -> Connection {
        let dir = tempdir().unwrap();
        let db_path: PathBuf = dir.path().join("test_categories.db");
        let conn = Connection::open(&db_path).unwrap();

        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS categories (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL,
                parent_id INTEGER REFERENCES categories(id),
                sort_order INTEGER DEFAULT 0,
                created_at TEXT DEFAULT (datetime('now', 'localtime')),
                updated_at TEXT DEFAULT (datetime('now', 'localtime'))
            );
            "#,
        )
        .unwrap();

        conn
    }

    fn count_categories(conn: &Connection) -> i64 {
        conn.query_row("SELECT COUNT(*) FROM categories", [], |row| row.get(0)).unwrap()
    }

    #[test]
    fn test_import_preset_categories_creates_hierarchy() {
        let conn = create_test_db_with_categories();

        let txt = "技术/前端开发/React生态\n技术/后端开发/数据库\n";

        let created = crate::db::import_preset_categories_into(&conn, txt).unwrap();

        // 6 个分类：技术, 前端开发, React生态, 后端开发, 数据库
        assert_eq!(created, 5);
        assert_eq!(count_categories(&conn), 5);

        // 验证层级：React生态 的父级是 前端开发，前端开发 的父级是 技术
        let react_id: i64 = conn.query_row(
            "SELECT id FROM categories WHERE name = 'React生态'",
            [],
            |row| row.get(0),
        ).unwrap();
        let react_parent: i64 = conn.query_row(
            "SELECT parent_id FROM categories WHERE id = ?",
            rusqlite::params![react_id],
            |row| row.get(0),
        ).unwrap();
        let frontend_name: String = conn.query_row(
            "SELECT name FROM categories WHERE id = ?",
            rusqlite::params![react_parent],
            |row| row.get(0),
        ).unwrap();
        assert_eq!(frontend_name, "前端开发");
    }

    #[test]
    fn test_import_preset_categories_is_idempotent() {
        let conn = create_test_db_with_categories();

        let txt = "技术/前端开发/React生态\n";

        let first = crate::db::import_preset_categories_into(&conn, txt).unwrap();
        assert_eq!(first, 3);

        // 第二次导入不应重复创建
        let second = crate::db::import_preset_categories_into(&conn, txt).unwrap();
        assert_eq!(second, 0);
        assert_eq!(count_categories(&conn), 3);
    }

    #[test]
    fn test_import_preset_categories_does_not_conflict_with_user_categories() {
        let conn = create_test_db_with_categories();

        // 模拟用户已有自定义分类（占用自增 id 1、2）
        conn.execute(
            "INSERT INTO categories (name, parent_id) VALUES ('用户自定义A', NULL)",
            [],
        ).unwrap();
        conn.execute(
            "INSERT INTO categories (name, parent_id) VALUES ('用户自定义B', NULL)",
            [],
        ).unwrap();
        let before = count_categories(&conn);
        assert_eq!(before, 2);

        let txt = "技术/前端开发/React生态\n";

        let created = crate::db::import_preset_categories_into(&conn, txt).unwrap();
        assert_eq!(created, 3);

        // 总数 = 2 用户 + 3 预置 = 5
        assert_eq!(count_categories(&conn), 5);

        // 用户分类应完好保留
        let user_a: i64 = conn.query_row(
            "SELECT COUNT(*) FROM categories WHERE name = '用户自定义A'",
            [],
            |row| row.get(0),
        ).unwrap();
        assert_eq!(user_a, 1);
    }

    #[test]
    fn test_import_preset_categories_skips_comments_and_empty_lines() {
        let conn = create_test_db_with_categories();

        let txt = "-- 注释\n# 另一种注释\n\n技术/前端开发\n   \n";

        let created = crate::db::import_preset_categories_into(&conn, txt).unwrap();
        assert_eq!(created, 2);
        assert_eq!(count_categories(&conn), 2);
    }
}
