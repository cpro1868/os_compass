use rusqlite::Connection;

fn main() {
    let style = "tech_blog";
    let templates = vec![
        ("tech_popular", "你是技术科普作家...\n\n项目名称：{project_name}\n..."),
        ("business", "你是商业文案...\n\n项目名称：{project_name}\n..."),
        ("tech_blog", include_str!("src/prompts/tech_blog.txt")),
        ("brief", include_str!("src/prompts/brief.txt")),
        ("humor", "你是幽默写手...\n\n项目名称：{project_name}\n..."),
    ];
    
    let template = templates.iter().find(|(id, _)| *id == style).map(|(_, t)| *t).unwrap_or("");
    
    println!("=== Style: {} ===", style);
    println!("Template length: {}", template.len());
    println!("Template preview:\n{}", &template[..template.len().min(500)]);
    
    // 模拟替换
    let prompt = template
        .replace("{project_name}", "测试项目")
        .replace("{description}", "测试描述")
        .replace("{use_cases}", "测试场景")
        .replace("{readme_excerpt}", "测试README")
        .replace("{word_count}", "1000-1500")
        .replace("{style}", "技术博客");
    
    println!("\n=== Generated prompt ===");
    println!("Length: {}", prompt.len());
    println!("Content:\n{}", prompt);
}
