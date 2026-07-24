use crate::llm::{LlmClient, LlmMessage};
use std::collections::HashMap;

const MAX_CHUNK_SIZE: usize = 5000;

fn split_into_chunks(text: &str, max_size: usize) -> Vec<String> {
    if text.len() <= max_size {
        return vec![text.to_string()];
    }

    let mut chunks = Vec::new();
    let mut current_pos = 0;
    let chars: Vec<char> = text.chars().collect();
    let total_chars = chars.len();

    while current_pos < total_chars {
        let remaining = total_chars - current_pos;
        let chunk_size = remaining.min(max_size);

        let mut end_pos = current_pos + chunk_size;
        if end_pos < total_chars {
            let chunk = &text[current_pos..end_pos];
            if let Some(last_newline) = chunk.rfind('\n') {
                end_pos = current_pos + last_newline;
            } else if let Some(last_space) = chunk.rfind(' ') {
                end_pos = current_pos + last_space;
            }
        }

        if end_pos <= current_pos {
            end_pos = (current_pos + chunk_size).min(total_chars);
        }

        chunks.push(text[current_pos..end_pos].to_string());
        current_pos = end_pos;
    }

    chunks
}

pub async fn translate_text(text: &str, target_lang: &str) -> Result<String, String> {
    if text.trim().is_empty() {
        return Ok(String::new());
    }

    let settings = crate::settings::get_settings();
    let engine = &settings.translate_engine;

    match engine.as_str() {
        "google" => {
            translate_with_google(text, target_lang).await
        }
        "llm" => {
            translate_text_with_llm(text, target_lang).await
        }
        _ => { // "auto" or default
            let chunks = split_into_chunks(text, MAX_CHUNK_SIZE);
            let chunk_count = chunks.len();

            if chunk_count == 1 {
                return translate_single_chunk(text, target_lang).await;
            }

            println!("[TRANSLATE] Text length: {}, split into {} chunks", text.len(), chunk_count);

            let mut translated_parts: Vec<String> = Vec::with_capacity(chunk_count);
            let mut errors: Vec<String> = Vec::new();

            for (i, chunk) in chunks.iter().enumerate() {
                print!("[TRANSLATE] Translating chunk {}/{}\r", i + 1, chunk_count);
                match translate_single_chunk(chunk, target_lang).await {
                    Ok(translated) => translated_parts.push(translated),
                    Err(e) => {
                        errors.push(format!("Chunk {}: {}", i, e));
                        translated_parts.push(chunk.clone());
                    }
                }
            }
            println!("[TRANSLATE] Done, {} errors", errors.len());

            Ok(translated_parts.join("\n"))
        }
    }
}

pub async fn translate_text_with_llm(text: &str, target_lang: &str) -> Result<String, String> {
    if text.trim().is_empty() {
        return Ok(String::new());
    }

    let chunks = split_into_chunks(text, MAX_CHUNK_SIZE);
    let chunk_count = chunks.len();
    
    if chunk_count == 1 {
        return translate_with_llm(text, target_lang).await;
    }

    println!("[TRANSLATE_LLM] Text length: {}, split into {} chunks", text.len(), chunk_count);
    
    let mut translated_parts: Vec<String> = Vec::with_capacity(chunk_count);
    
    for (i, chunk) in chunks.iter().enumerate() {
        print!("[TRANSLATE_LLM] Translating chunk {}/{}\r", i + 1, chunk_count);
        match translate_with_llm(chunk, target_lang).await {
            Ok(translated) => translated_parts.push(translated),
            Err(e) => {
                println!("[TRANSLATE_LLM] Chunk {} failed: {}", i, e);
                translated_parts.push(chunk.clone());
            }
        }
    }
    println!("[TRANSLATE_LLM] Done");

    Ok(translated_parts.join("\n"))
}

async fn translate_single_chunk(text: &str, target_lang: &str) -> Result<String, String> {
    let google_result = translate_with_google(text, target_lang).await;
    
    if google_result.is_ok() {
        return google_result;
    }
    
    println!("[TRANSLATE] Google translate failed, falling back to LLM");
    translate_with_llm(text, target_lang).await
}

async fn translate_with_google(text: &str, target_lang: &str) -> Result<String, String> {
    let source_lang = detect_google_lang(text)?;
    
    if source_lang == target_lang {
        return Ok(text.to_string());
    }

    let target_code = match target_lang {
        "zh-CN" | "zh" => "zh-CN",
        "en" | "en-US" => "en",
        "ja" => "ja",
        "ko" => "ko",
        "es" => "es",
        "fr" => "fr",
        "de" => "de",
        "ru" => "ru",
        _ => target_lang,
    };

    let encoded_text = urlencoding::encode(text);
    let url = format!(
        "https://translate.googleapis.com/translate_a/single?client=gtx&sl={}&tl={}&dt=t&q={}",
        source_lang, target_code, encoded_text
    );

    let mut client_builder = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30));

    let proxy_config: Option<(String, String, i32, String, String, bool)> = {
        let cfg = crate::settings::get_settings();
        Some((cfg.google_proxy_protocol, cfg.google_proxy_host, cfg.google_proxy_port, cfg.google_proxy_username, cfg.google_proxy_password, cfg.google_proxy_enabled))
    };

    if let Some((protocol, host, port, username, password, enabled)) = proxy_config {
        if enabled && !host.is_empty() && port > 0 {
            let proxy_url = if username.is_empty() {
                format!("{}://{}:{}", protocol, host, port)
            } else {
                format!("{}://{}:{}@{}:{}", protocol, username, password, host, port)
            };
            if let Ok(proxy) = reqwest::Proxy::all(&proxy_url) {
                client_builder = client_builder.proxy(proxy);
            }
        }
    }

    let client = client_builder.build().map_err(|e| format!("Failed to create client: {}", e))?;
    let response = client.get(&url).send().await
        .map_err(|e| format!("HTTP request failed: {}", e))?;
    
    let body: serde_json::Value = response.json().await
        .map_err(|e| format!("JSON parse failed: {}", e))?;

    let translations = body.get(0)
        .and_then(|arr| arr.as_array())
        .ok_or("Invalid response format")?;

    let mut result = String::new();
    for item in translations {
        if let Some(translated_text) = item.get(0).and_then(|t| t.as_str()) {
            result.push_str(translated_text);
        }
    }

    if result.is_empty() {
        return Err("Empty translation result".to_string());
    }

    Ok(result)
}

fn detect_google_lang(text: &str) -> Result<String, String> {
    let sample = if text.len() > 100 { &text[..100] } else { text };
    let sample_lower = sample.to_lowercase();
    
    if sample_lower.chars().any(|c| c >= '\u{4e00}' && c <= '\u{9fff}') {
        return Ok("zh-CN".to_string());
    }
    if sample_lower.contains("function") || sample_lower.contains("const") || 
       sample_lower.contains("let") || sample_lower.contains("var") ||
       sample_lower.contains("import") || sample_lower.contains("export") {
        return Ok("en".to_string());
    }
    if sample_lower.contains("function") || sample_lower.contains("def") ||
       sample_lower.contains("import") || sample_lower.contains("class") {
        if sample_lower.contains("def") || sample_lower.contains(":") {
            return Ok("python".to_string());
        }
        return Ok("en".to_string());
    }
    if sample_lower.contains("¿") || sample_lower.contains("ñ") || 
       sample_lower.contains("á") || sample_lower.contains("é") {
        return Ok("es".to_string());
    }
    if sample_lower.contains("é") || sample_lower.contains("è") || 
       sample_lower.contains("ç") || sample_lower.contains("à") {
        return Ok("fr".to_string());
    }
    if sample_lower.contains("ä") || sample_lower.contains("ö") || 
       sample_lower.contains("ü") || sample_lower.contains("ß") {
        return Ok("de".to_string());
    }
    if sample_lower.chars().any(|c| c >= '\u{3040}' && c <= '\u{309f}') {
        return Ok("ja".to_string());
    }
    if sample_lower.chars().any(|c| c >= '\u{ac00}' && c <= '\u{d7af}') {
        return Ok("ko".to_string());
    }
    
    Ok("auto".to_string())
}

fn clean_llm_output(text: &str) -> String {
    let mut result = text.to_string();
    
    let markers = [
        ("<think>", "</think>"),
        ("<｜", "｜>"),
        ("<|", "|>"),
        ("k>", ""),
    ];
    
    for (start_marker, end_marker) in markers {
        loop {
            if let Some(s) = result.find(start_marker) {
                if let Some(e) = result[s..].find(end_marker) {
                    let end_pos = s + e + end_marker.len();
                    result = format!("{}{}", &result[..s], &result[end_pos..]);
                } else {
                    break;
                }
            } else {
                break;
            }
        }
    }
    
    result.trim().to_string()
}

async fn translate_with_llm(text: &str, target_lang: &str) -> Result<String, String> {
    println!("[TRANSLATE_LLM] Start, text length: {}, target: {}", text.len(), target_lang);
    let client = LlmClient::from_settings()
        .ok_or_else(|| "LLM not configured".to_string())?;
    println!("[TRANSLATE_LLM] LLM client created");

    let prompt = format!(
        r#"Translate the following text to {}. Return ONLY the translated text without any explanations, quotes, or markers.

Text:
{}
"#,
        target_lang, text
    );

    let messages = vec![
        LlmMessage {
            role: "system".to_string(),
            content: "You are a professional translator. Translate accurately and naturally. Preserve formatting.".to_string(),
        },
        LlmMessage {
            role: "user".to_string(),
            content: prompt,
        },
    ];

    println!("[TRANSLATE_LLM] Calling LLM chat...");
    let result = client.chat(messages).await?;
    println!("[TRANSLATE_LLM] LLM chat success, result length: {}", result.len());
    Ok(clean_llm_output(&result))
}

pub async fn detect_language(text: &str) -> Result<String, String> {
    if text.trim().is_empty() {
        return Ok("unknown".to_string());
    }

    let client = LlmClient::from_settings()
        .ok_or_else(|| "LLM not configured".to_string())?;

    let sample = if text.len() > 500 {
        &text[..500]
    } else {
        text
    };

    let prompt = format!(
        r#"Detect the language of the following text. Return ONLY the ISO 639-1 language code (e.g., 'en', 'zh', 'ja', 'ko').

Text:
{}
"#,
        sample
    );

    let messages = vec![
        LlmMessage {
            role: "user".to_string(),
            content: prompt,
        },
    ];

    let result = client.chat(messages).await?;
    Ok(result.trim().to_lowercase())
}

pub fn needs_translation(source_lang: Option<&str>, target_lang: &str) -> bool {
    let source = source_lang.unwrap_or("en");
    let target = target_lang.to_lowercase();
    
    source.to_lowercase() != target && source.to_lowercase() != "unknown"
}
