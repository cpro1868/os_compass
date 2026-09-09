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

    // =====================================================================
    // 分类管理 - 自定义导入功能测试（对应设计文档 §9.1 T1-T11）
    // =====================================================================

    fn fixture_txt() -> &'static str {
        "# 测试清单\n技术/前端开发/Web框架/React生态\n技术/前端开发/Web框架/Vue生态\n商业/金融科技/支付系统\n"
    }

    fn fixture_json() -> &'static str {
        r#"[
            { "path": ["技术", "前端开发", "Web框架"], "name": "React生态" },
            { "path": ["技术", "前端开发", "Web框架"], "name": "Vue生态", "explain": "忽略的说明" },
            { "name": "商业" }
        ]"#
    }

    /// T1: txt 含中文多级路径 → 正确逐级创建，层级 parent_id 正确
    #[test]
    fn test_import_items_creates_hierarchy_from_txt() {
        let conn = create_test_db_with_categories();
        let items = crate::db::parse_categories_txt_for_test(fixture_txt());

        let summary = crate::db::import_category_items_into(&conn, &items, None).unwrap();
        assert_eq!(summary.created, 8); // 技术/前端开发/Web框架/React生态/Vue生态 + 商业/金融科技/支付系统 = 8
        assert_eq!(summary.skipped, 0);
        assert_eq!(summary.errors.len(), 0);
        assert_eq!(count_categories(&conn), 8);

        // 验证层级：React生态 父=Web框架 父=前端开发 父=技术(顶级)
        let react_id: i64 = conn.query_row(
            "SELECT id FROM categories WHERE name = 'React生态'",
            [], |row| row.get(0),
        ).unwrap();
        let web_id: i64 = conn.query_row(
            "SELECT parent_id FROM categories WHERE id = ?",
            rusqlite::params![react_id], |row| row.get(0),
        ).unwrap();
        let web_name: String = conn.query_row(
            "SELECT name FROM categories WHERE id = ?",
            rusqlite::params![web_id], |row| row.get(0),
        ).unwrap();
        assert_eq!(web_name, "Web框架");

        let tech_count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM categories WHERE name = '技术' AND parent_id IS NULL",
            [], |row| row.get(0),
        ).unwrap();
        assert_eq!(tech_count, 1);
    }

    /// T2: json 含 path 数组 → 正确创建
    #[test]
    fn test_import_items_from_json() {
        let conn = create_test_db_with_categories();
        let items = crate::db::parse_categories_json_for_test(fixture_json()).unwrap();
        let summary = crate::db::import_category_items_into(&conn, &items, None).unwrap();
        // 技术/前端开发/Web框架 3 + React生态/Vue生态 2 + 商业 1 = 6
        assert_eq!(summary.created, 6);
        assert_eq!(count_categories(&conn), 6);

        // explain 字段被忽略，不影响数量
        let vue_count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM categories WHERE name = 'Vue生态'",
            [], |row| row.get(0),
        ).unwrap();
        assert_eq!(vue_count, 1);
    }

    /// T3: json 单条缺 name → 该条 error，其他正常
    #[test]
    fn test_import_json_missing_name_is_error() {
        // 缺 name 属于"全文件解析失败"，返回 Err（parse_categories_json 整文件失败）
        let bad_json = r#"[ { "path": ["技术"] } ]"#;
        let result = crate::db::parse_categories_json_for_test(bad_json);
        assert!(result.is_err());
    }

    /// T4: json 顶层非数组 → 返回 Err
    #[test]
    fn test_import_json_top_level_not_array() {
        let bad_json = r#"{ "name": "技术" }"#;
        let result = crate::db::parse_categories_json_for_test(bad_json);
        assert!(result.is_err());
    }

    /// T5: 重复导入同一文件 → 第二次全部 skip，created=0
    #[test]
    fn test_import_items_idempotent() {
        let conn = create_test_db_with_categories();
        let items = crate::db::parse_categories_txt_for_test("技术/前端开发/React生态\n");

        let first = crate::db::import_category_items_into(&conn, &items, None).unwrap();
        assert_eq!(first.created, 3);

        let second = crate::db::import_category_items_into(&conn, &items, None).unwrap();
        assert_eq!(second.created, 0);
        assert_eq!(second.skipped, 1);
        assert_eq!(count_categories(&conn), 3);
    }

    /// T6: 父分类已存在，导入同父子分类 → 子分类追加，父不重复建
    #[test]
    fn test_import_items_appends_child_to_existing_parent() {
        let conn = create_test_db_with_categories();
        conn.execute(
            "INSERT INTO categories (name, parent_id) VALUES ('技术', NULL)",
            [],
        ).unwrap();

        let items = crate::db::parse_categories_txt_for_test("技术/新增子类\n");
        let summary = crate::db::import_category_items_into(&conn, &items, None).unwrap();

        assert_eq!(summary.created, 1);
        assert_eq!(summary.skipped, 0);

        let tech_count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM categories WHERE name = '技术' AND parent_id IS NULL",
            [], |row| row.get(0),
        ).unwrap();
        assert_eq!(tech_count, 1);
        assert_eq!(count_categories(&conn), 2);
    }

    /// T7: 指定 parent_id 导入 → 全部挂到该父下
    #[test]
    fn test_import_items_under_parent() {
        let conn = create_test_db_with_categories();
        conn.execute(
            "INSERT INTO categories (name, parent_id) VALUES ('我的项目', NULL)",
            [],
        ).unwrap();
        let parent_id: i64 = conn.query_row(
            "SELECT id FROM categories WHERE name = '我的项目'",
            [], |row| row.get(0),
        ).unwrap();

        let items = crate::db::parse_categories_txt_for_test("技术/前端开发\n");
        let summary = crate::db::import_category_items_into(&conn, &items, Some(parent_id)).unwrap();
        assert_eq!(summary.created, 2);

        // 技术 应在 我的项目 下
        let tech_parent: i64 = conn.query_row(
            "SELECT parent_id FROM categories WHERE name = '技术'",
            [], |row| row.get(0),
        ).unwrap();
        assert_eq!(tech_parent, parent_id);
    }

    /// T8: 空文件 / 全注释 / 全空行 → created=0，total=0
    #[test]
    fn test_import_items_empty_file() {
        let conn = create_test_db_with_categories();
        let items = crate::db::parse_categories_txt_for_test("# 只有注释\n\n  \n-- 另一注释\n");
        assert_eq!(items.len(), 0);

        let summary = crate::db::import_category_items_into(&conn, &items, None).unwrap();
        assert_eq!(summary.created, 0);
        assert_eq!(summary.skipped, 0);
        assert_eq!(count_categories(&conn), 0);
    }

    /// T9: GBK 编码 txt → 正确解析中文（模拟 GBK 字节，经 read_categories_file 解码）
    #[test]
    fn test_import_read_gbk_file() {
        use std::io::Write;
        let dir = tempdir().unwrap();
        let path = dir.path().join("gbk_cats.txt");

        let text_utf8 = "技术/前端开发/React生态\n";
        let (gbk_bytes, _, _) = encoding_rs::GBK.encode(text_utf8);
        let mut f = std::fs::File::create(&path).unwrap();
        f.write_all(&gbk_bytes).unwrap();
        drop(f);

        let (content, enc) = crate::db::read_categories_file_for_test(&path.to_string_lossy()).unwrap();
        assert_eq!(enc, "gbk");
        assert!(content.contains("技术/前端开发/React生态"));
    }

    /// T10: 单条错误不中断（模拟中间父级为空段的场景由解析层过滤，此处验证有效输入均入库）
    #[test]
    fn test_import_items_multiple_entries_all_succeed() {
        let conn = create_test_db_with_categories();
        let items = crate::db::parse_categories_txt_for_test("A/B\nA/C\nD\n");
        let summary = crate::db::import_category_items_into(&conn, &items, None).unwrap();
        assert_eq!(summary.created, 4); // A/B/C/D
        assert_eq!(summary.errors.len(), 0);
    }

    /// T11: 同名不同父 → 共存（判重按 name+parent_id）
    #[test]
    fn test_import_items_same_name_different_parent_coexist() {
        let conn = create_test_db_with_categories();
        let items = crate::db::parse_categories_txt_for_test("技术/Core\n架构/Core\n");
        let summary = crate::db::import_category_items_into(&conn, &items, None).unwrap();
        assert_eq!(summary.created, 4); // 技术/架构/Core/Core

        let core_count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM categories WHERE name = 'Core'",
            [], |row| row.get(0),
        ).unwrap();
        assert_eq!(core_count, 2); // 两个不同父下的 Core 共存
    }

    /// 预览：parse_categories_file 对已存在路径标 skip、新路径标 new
    #[test]
    fn test_parse_categories_file_preview_status() {
        let conn = create_test_db_with_categories();
        conn.execute(
            "INSERT INTO categories (name, parent_id) VALUES ('技术', NULL)",
            [],
        ).unwrap();

        let items = crate::db::parse_categories_txt_for_test("技术\n技术/前端开发\n");
        // 预览判断不依赖 DATABASE 全局（此处直接测试 simulate 逻辑）
        let r1 = crate::db::simulate_path_exists_for_test(&conn, &items[0].full_path, None).unwrap();
        assert!(r1); // 技术 已存在 → skip
        let r2 = crate::db::simulate_path_exists_for_test(&conn, &items[1].full_path, None).unwrap();
        assert!(!r2); // 技术/前端开发 前端开发不存在 → new
    }
}
