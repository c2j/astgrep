//! Interactive playground handler for testing the API

use axum::response::Html;
use crate::WebResult;

/// Interactive playground endpoint
pub async fn playground() -> WebResult<Html<String>> {
    let base = include_str!("playground/template.html");

    let guide_md = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../docs/astgrep-Guide.md"));
    let guide_md_json = serde_json::to_string(guide_md).unwrap_or_else(|_| "\"\"".to_string());
    let html = base.replace("__GUIDE_MD__", &guide_md_json);
    Ok(Html(html))
}

#[cfg(test)]
mod tests;
