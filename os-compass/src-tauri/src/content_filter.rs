use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilterResult {
    pub flagged: bool,
    pub score: i64,
    pub matched_words: Vec<String>,
}

const ZH_SPAM_PATTERNS: &[&str] = &[
    "加微信", "扫码关注", "二维码", "加Q", "加群",
    "联系我", "收徒", "培训", "课程", "学费",
    "优惠券", "立即购买", "点击购买", "限时优惠", "福利",
    "免费送", "送资料", "送课程", "进群", "私聊",
    "代做", "代写", "代开发", "兼职", "副业",
    "日赚", "月入", "稳赚", "高收益", "投资",
    "赌博", "色情", "成人网站", "博彩",
    "赚钱APP", "分红盘", "资金盘", "传销",
];

const EN_SPAM_PATTERNS: &[&str] = &[
    "buy now", "click here", "free money", "make money fast",
    "act now", "limited time", "offer expires", "congratulations",
    "winner", "lottery", "casino", "porn", "xxx",
];

pub fn scan_content(content: &str, threshold: i64) -> FilterResult {
    let content_lower = content.to_lowercase();
    let mut matched_words = Vec::new();
    let mut total_score: i64 = 0;

    for pattern in ZH_SPAM_PATTERNS.iter().chain(EN_SPAM_PATTERNS.iter()) {
        let pattern_lower = pattern.to_lowercase();
        if content_lower.contains(&pattern_lower) {
            total_score += 10;
            matched_words.push(pattern.to_string());
        }
    }

    FilterResult {
        flagged: total_score >= threshold,
        score: total_score,
        matched_words,
    }
}

pub fn should_filter_content(content: &str, threshold: i64) -> bool {
    let result = scan_content(content, threshold);
    result.flagged
}

pub fn filter_radar_item(
    title: &str,
    content: &str,
    threshold: i64,
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
    pub fn threshold(&self) -> i64 {
        match self {
            FilterLevel::Off => i64::MAX,
            FilterLevel::Low => 30,
            FilterLevel::Medium => 20,
            FilterLevel::High => 10,
        }
    }
}
