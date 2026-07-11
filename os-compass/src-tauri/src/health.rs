use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectMetrics {
    pub stars: i32,
    pub forks: i32,
    pub open_issues: i32,
    pub has_readme: bool,
    pub has_license: bool,
    pub has_homepage: bool,
    pub primary_language: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthScore {
    pub overall: f64,
    pub activity: f64,
    pub popularity: f64,
    pub maintenance: f64,
    pub documentation: f64,
}

pub fn calculate_health_score(metrics: ProjectMetrics) -> HealthScore {
    let activity = calculate_activity_score(&metrics);
    let popularity = calculate_popularity_score(metrics.stars, metrics.forks);
    let maintenance = calculate_maintenance_score(metrics.open_issues, metrics.stars);
    let documentation = calculate_documentation_score(
        metrics.has_readme,
        metrics.has_license,
        metrics.has_homepage,
    );

    let raw_overall = activity * 0.25 + popularity * 0.30 + maintenance * 0.25 + documentation * 0.20;
    let overall = (raw_overall * 100.0).round();

    println!("Health score calculation - activity: {:.2}, popularity: {:.2}, maintenance: {:.2}, documentation: {:.2}, raw: {:.3}, final: {:.0}",
        activity, popularity, maintenance, documentation, raw_overall, overall);

    HealthScore {
        overall,
        activity: (activity * 100.0).round(),
        popularity: (popularity * 100.0).round(),
        maintenance: (maintenance * 100.0).round(),
        documentation: (documentation * 100.0).round(),
    }
}

fn calculate_activity_score(metrics: &ProjectMetrics) -> f64 {
    let stars_factor = (metrics.stars as f64).ln().min(10.0) / 10.0;
    let forks_factor = (metrics.forks as f64).ln().min(8.0) / 8.0;
    
    (stars_factor * 0.6 + forks_factor * 0.4).min(1.0)
}

fn calculate_popularity_score(stars: i32, forks: i32) -> f64 {
    if stars == 0 && forks == 0 {
        return 0.0;
    }
    
    let stars_score = if stars > 10000 {
        1.0
    } else if stars > 1000 {
        0.8
    } else if stars > 100 {
        0.5
    } else if stars > 10 {
        0.3
    } else {
        0.1
    };
    
    let forks_ratio = if stars > 0 {
        (forks as f64 / stars as f64).min(1.0)
    } else {
        0.0
    };
    
    (stars_score * 0.7 + forks_ratio * 0.3).min(1.0)
}

fn calculate_maintenance_score(open_issues: i32, stars: i32) -> f64 {
    if stars == 0 {
        return 0.0;
    }
    
    let issue_ratio = open_issues as f64 / stars as f64;
    
    if issue_ratio < 0.05 {
        1.0
    } else if issue_ratio < 0.1 {
        0.8
    } else if issue_ratio < 0.2 {
        0.6
    } else if issue_ratio < 0.5 {
        0.4
    } else {
        0.2
    }
}

fn calculate_documentation_score(has_readme: bool, has_license: bool, has_homepage: bool) -> f64 {
    let mut score = 0.0;
    if has_readme { score += 0.4; }
    if has_license { score += 0.3; }
    if has_homepage { score += 0.3; }
    score
}

pub fn health_score_to_rating(score: f64) -> &'static str {
    if score >= 80.0 {
        "Excellent"
    } else if score >= 60.0 {
        "Good"
    } else if score >= 40.0 {
        "Fair"
    } else if score >= 20.0 {
        "Poor"
    } else {
        "Very Poor"
    }
}
