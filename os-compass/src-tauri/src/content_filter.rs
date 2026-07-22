use banlex::{default_lexicon, Lexicon, NormalizeConfig, Scanner, ScanResult};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilterResult {
    pub flagged: bool,
    pub score: f64,
    pub matched_words: Vec<String>,
}

fn get_zh_spam_lexicon() -> Lexicon {
    let json = include_str!("../resources/zh_spam_lexicon.json");
    Lexicon::from_json(json).unwrap_or_else(|_| default_lexicon())
}

static ZH_SPAM_LEXICON: Lazy<Lexicon> = Lazy::new(get_zh_spam_lexicon);

pub fn create_scanner(threshold: f64) -> Scanner {
    Scanner::builder(ZH_SPAM_LEXICON.clone(), threshold)
        .normalize(NormalizeConfig::default())
        .build()
}

pub fn scan_content(content: &str, threshold: f64) -> FilterResult {
    let scanner = create_scanner(threshold);
    let result = scanner.scan(content);
    
    let matched_words: Vec<String> = result.matches.iter()
        .map(|m| m.term.clone())
        .collect();
    
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
