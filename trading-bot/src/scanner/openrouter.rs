use crate::config::AppConfig;

/// OpenRouter attribution headers (HTTP-Referer / X-Title) used for
/// rankings/attribution. Both are opt-in via env — omitted when empty.
pub fn openrouter_attribution_headers(cfg: &AppConfig) -> Vec<(&'static str, String)> {
    let mut headers = Vec::new();
    if !cfg.openrouter_http_referer.is_empty() {
        headers.push(("HTTP-Referer", cfg.openrouter_http_referer.clone()));
    }
    if !cfg.openrouter_app_title.is_empty() {
        headers.push(("X-Title", cfg.openrouter_app_title.clone()));
    }
    headers
}
