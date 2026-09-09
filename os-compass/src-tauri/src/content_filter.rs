use banlex::{default_lexicon, Lexicon, NormalizeConfig, Scanner, ScanResult};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::Mutex;
use log;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilterResult {
    pub flagged: bool,
    pub score: f64,
    pub matched_words: Vec<String>,
}

fn get_default_lexicon() -> Lexicon {
    let default_json = r#"{
  "name": "default-spam-lexicon",
  "entries": [
    { "term": "加微信", "category": "spam", "weight": 2.0, "lang": "zh" },
    { "term": "博彩", "category": "spam", "weight": 2.0, "lang": "zh" },
    { "term": "彩票", "category": "spam", "weight": 1.5, "lang": "zh" },
    { "term": "百家乐", "category": "spam", "weight": 2.0, "lang": "zh" },
    { "term": "菠菜", "category": "spam", "weight": 2.0, "lang": "zh" },
    { "term": "代理", "category": "spam", "weight": 1.0, "lang": "zh" },
    { "term": "首存", "category": "spam", "weight": 2.0, "lang": "zh" },
    { "term": "返利", "category": "spam", "weight": 2.0, "lang": "zh" },
    { "term": "t.me", "category": "spam", "weight": 2.0, "lang": "en" },
    { "term": "casino", "category": "spam", "weight": 2.0, "lang": "en" }
  ]
}"#;
    Lexicon::from_json(default_json).unwrap_or_else(|_| default_lexicon())
}

static SPAM_LEXICON: Lazy<Mutex<Lexicon>> = Lazy::new(|| {
    Mutex::new(get_default_lexicon())
});

pub fn load_lexicon_from_file(path: &Path) -> bool {
    if path.exists() {
        if let Ok(content) = std::fs::read_to_string(path) {
            if let Ok(lexicon) = Lexicon::from_json(&content) {
                if let Ok(mut guard) = SPAM_LEXICON.lock() {
                    *guard = lexicon;
                    log::debug!("[LEXICON] Loaded from: {:?}", path);
                    return true;
                }
            }
        }
    }
    log::warn!("[LEXICON] Failed to load from: {:?}", path);
    false
}

pub fn create_scanner(threshold: f64) -> Scanner {
    let lexicon = SPAM_LEXICON.lock().unwrap().clone();
    Scanner::builder(lexicon, threshold)
        .normalize(NormalizeConfig::default())
        .build()
}

pub fn scan_content(content: &str, threshold: f64) -> FilterResult {
    let scanner = create_scanner(threshold);
    let result = scanner.scan(content);
    
    let matched_words: Vec<String> = result.matches.iter()
        .map(|m| m.term.clone())
        .collect();
    
    if !matched_words.is_empty() {
        log::debug!("[MATCH] words={:?}, score={:.2}, threshold={}, flagged={}", 
            matched_words, result.total_score, threshold, result.flagged);
    }
    
    FilterResult {
        flagged: result.flagged,
        score: result.total_score,
        matched_words,
    }
}

pub fn should_filter_content(content: &str, threshold: f64) -> bool {
    let result = scan_content(content, threshold);
    result.flagged
}

pub fn filter_radar_item(
    title: &str,
    content: &str,
    threshold: f64,
) -> bool {
    let combined = format!("{} {}", title, content);
    should_filter_content(&combined, threshold)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum FilterLevel {
    #[default]
    Off = 0,
    Low = 1,
    Medium = 2,
    High = 3,
}

impl FilterLevel {
    pub fn threshold(&self) -> f64 {
        match self {
            FilterLevel::Off => f64::MAX,
            FilterLevel::Low => 3.0,
            FilterLevel::Medium => 2.0,
            FilterLevel::High => 1.0,
        }
    }
}
