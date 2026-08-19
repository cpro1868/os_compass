use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct JsonObj {
    #[serde(rename = "summary")]
    summary: Option<String>,
    #[serde(rename = "use_cases")]
    use_cases: Option<String>,
    #[serde(rename = "useCases")]
    use_cases_alt: Option<String>,
}

fn extract_json_field(json: &str, field: &str) -> Option<String> {
    match serde_json::from_str::<JsonObj>(json) {
        Ok(obj) => {
            match field {
                "summary" => obj.summary.or(obj.use_cases).or(obj.use_cases_alt),
                _ => None,
            }
        }
        Err(e) => {
            println!("JSON parse error: {}", e);
            None
        }
    }
}

fn main() {
    // 模拟 LLM 返回的各种 JSON 格式
    let test_cases = vec![
        r#"{"summary": "这是一个测试项目", "use_cases": "用于Web开发", "risks": "无", "dependencies": "Node.js"}"#,
        r#"{"summary":"简洁格式"}"#,
        r#"{"useCases":"驼峰格式"}"#,
        r#"{"use_cases":"下划线格式"}"#,
        r#"{"summary": "带空格"}"#,
        r#"{"summary": "带\n换行"}"#,
        r#"{"summary": "带\"引号"}"#,
    ];

    for (i, json) in test_cases.iter().enumerate() {
        println!("\n=== Test case {} ===", i + 1);
        println!("Input: {}", json);
        let result = extract_json_field(json, "summary");
        println!("Result: {:?}", result);
    }
}
