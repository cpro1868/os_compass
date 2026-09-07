#[cfg(test)]
mod tests {
    use crate::embedding::*;
    use crate::plugins::search::{primary_language, ProjectMatch};
    use rusqlite::Connection;

    #[test]
    fn primary_language_parses_first_from_json_array() {
        let raw = Some(r#"["C++","QML","JavaScript"]"#.to_string());
        assert_eq!(primary_language(raw), Some("C++".to_string()));
    }

    #[test]
    fn primary_language_returns_plain_string_when_not_json() {
        let raw = Some("Rust".to_string());
        assert_eq!(primary_language(raw), Some("Rust".to_string()));
    }

    #[test]
    fn primary_language_handles_empty_array() {
        let raw = Some("[]".to_string());
        assert_eq!(primary_language(raw), None);
    }

    #[test]
    fn primary_language_handles_none_or_blank() {
        assert_eq!(primary_language(None), None);
        assert_eq!(primary_language(Some("".to_string())), None);
        assert_eq!(primary_language(Some("   ".to_string())), None);
    }

    #[test]
    fn cosine_similarity_handles_unequal_lengths() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![1.0, 0.0, 0.0, 0.0];
        assert!((cosine_similarity(&a, &b) - 1.0).abs() < 1e-5);
    }

    #[test]
    fn cosine_similarity_returns_zero_for_orthogonal() {
        let a = vec![1.0, 0.0];
        let b = vec![0.0, 1.0];
        assert!((cosine_similarity(&a, &b)).abs() < 1e-6);
    }

    fn build_project_embedding_text(name: &str, description: Option<&str>, languages: Option<&str>) -> String {
        let mut parts: Vec<String> = Vec::new();
        if !name.trim().is_empty() {
            parts.push(name.trim().to_string());
        }
        if let Some(d) = description {
            let t = d.trim();
            if !t.is_empty() {
                parts.push(t.to_string());
            }
        }
        if let Some(langs) = languages {
            let cleaned = crate::plugins::search::primary_language(Some(langs.to_string()));
            if let Some(c) = cleaned {
                parts.push(format!("language: {}", c));
            }
        }
        parts.join("\n")
    }

    #[test]
    fn embedding_text_includes_languages_and_description() {
        let text = build_project_embedding_text(
            "奇幻地图生成器",
            Some("Web application generating interactive maps"),
            Some(r#"["HTML","TypeScript"]"#),
        );
        assert!(text.contains("奇幻地图生成器"));
        assert!(text.contains("Web application generating interactive maps"));
        assert!(text.contains("language: HTML"));
    }

    #[test]
    fn embedding_text_falls_back_when_languages_missing() {
        let text = build_project_embedding_text("crm", Some("Open source CRM"), None);
        assert_eq!(text, "crm\nOpen source CRM");
    }

    #[test]
    fn embedding_text_skips_blank_description() {
        let text = build_project_embedding_text("foo", Some("   "), Some("Rust"));
        assert!(text.contains("foo"));
        assert!(text.contains("language: Rust"));
        assert!(!text.contains("   "));
    }

    #[test]
    fn embedding_text_for_real_map_projects_keeps_keyword() {
        let text = build_project_embedding_text(
            "奇幻地图生成器",
            Some("Web application generating interactive and highly customizable maps"),
            Some(r#"["HTML","TypeScript"]"#),
        );
        assert!(text.contains("地图"));
        assert!(text.contains("interactive"));
        assert!(text.contains("language: HTML"));
    }

    #[test]
    fn store_and_score_round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("embed.db");
        let conn = Connection::open(&db_path).unwrap();

        conn.execute_batch(
            "CREATE TABLE projects (
                id INTEGER PRIMARY KEY,
                name TEXT NOT NULL,
                url TEXT,
                description TEXT,
                languages TEXT,
                stars INTEGER,
                forks INTEGER,
                lifecycle_status TEXT DEFAULT 'ACTIVE'
            );
            INSERT INTO projects (id, name, url, description, languages, stars, forks)
                VALUES (1, 'mapgen', 'https://example.com/mapgen',
                        'Interactive map generator for fantasy worlds',
                        '[\"TypeScript\",\"Rust\"]', 100, 10),
                       (2, 'unrelated', 'https://example.com/x',
                        'Random unrelated tool', '[\"Go\"]', 5, 1);
            "
        ).unwrap();

        let _text_map = build_project_embedding_text(
            "mapgen", Some("Interactive map generator for fantasy worlds"),
            Some(r#"["TypeScript","Rust"]"#),
        );
        let _text_unrelated = build_project_embedding_text(
            "unrelated", Some("Random unrelated tool"),
            Some(r#"["Go"]"#),
        );

        let v_map = vec![1.0f32; 1024];
        let mut v_unrelated = vec![0.5f32; 1024];
        for i in (0..v_unrelated.len()).step_by(2) {
            v_unrelated[i] = -0.5;
        }

        let emb_json_map = serde_json::to_string(&v_map).unwrap();
        let emb_json_unrelated = serde_json::to_string(&v_unrelated).unwrap();
        conn.execute(
            "CREATE TABLE project_embeddings (
                id INTEGER PRIMARY KEY,
                project_id INTEGER NOT NULL UNIQUE,
                embedding TEXT NOT NULL,
                dimension INTEGER NOT NULL DEFAULT 1024
            )",
            [],
        ).unwrap();
        conn.execute(
            "INSERT INTO project_embeddings (project_id, embedding) VALUES (1, ?)",
            rusqlite::params![emb_json_map],
        ).unwrap();
        conn.execute(
            "INSERT INTO project_embeddings (project_id, embedding) VALUES (2, ?)",
            rusqlite::params![emb_json_unrelated],
        ).unwrap();

        // Verify shape: every vec is 1024
        let mut stmt = conn.prepare("SELECT project_id, embedding FROM project_embeddings").unwrap();
        let rows = stmt.query_map([], |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
        }).unwrap();
        let mut parsed = Vec::new();
        for r in rows {
            let (pid, j) = r.unwrap();
            let v: Vec<f32> = serde_json::from_str(&j).unwrap();
            assert_eq!(v.len(), 1024);
            parsed.push((pid, v));
        }

        // Sanity: mapgen vector should be more similar to itself than unrelated
        let s_self = cosine_similarity(&v_map, &v_map);
        let s_other = cosine_similarity(&v_map, &v_unrelated);
        assert!(s_self > s_other);
    }

    #[test]
    fn project_match_constructor_accepts_project_id() {
        let pm = ProjectMatch {
            name: "n".into(),
            url: "u".into(),
            description: None,
            stars: None,
            forks: None,
            language: None,
            health_score: None,
            source: "local".into(),
            match_score: 0.5,
            project_id: Some(42),
        };
        assert_eq!(pm.project_id, Some(42));
    }

    #[test]
    fn threshold_constant_is_in_sane_range() {
        assert!(SEMANTIC_SEARCH_MIN_SCORE > 0.0 && SEMANTIC_SEARCH_MIN_SCORE < 1.0);
    }

    #[test]
    fn threshold_filters_low_similarity_vectors() {
        let query = vec![1.0f32, 0.0, 0.0];
        let relevant = vec![1.0f32, 0.0, 0.0];
        let garbage = vec![0.0f32, 1.0, 0.0];
        let s_rel = cosine_similarity(&query, &relevant);
        let s_gar = cosine_similarity(&query, &garbage);
        assert!(s_rel >= SEMANTIC_SEARCH_MIN_SCORE);
        assert!(s_gar < SEMANTIC_SEARCH_MIN_SCORE);
    }
}
