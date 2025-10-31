//! API documentation handler

use axum::response::Html;
use crate::WebResult;

/// API documentation endpoint
pub async fn api_docs() -> WebResult<Html<String>> {
    let version = env!("CARGO_PKG_VERSION");
    let html = format!(
        r#"<!DOCTYPE html>
<html>
<head>
    <meta charset="utf-8"/>
    <title>astgrep API Documentation</title>
    <style>
        body {{ font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Arial, sans-serif; margin: 40px; line-height: 1.6; }}
        .header {{ color: #2c3e50; }}
        .endpoint {{ background: #f8f9fa; padding: 15px; margin: 15px 0; }}
        .method {{ font-weight: bold; color: #007bff; }}
        .path {{ font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, "Liberation Mono", "Courier New", monospace; background: #e9ecef; padding: 2px 6px; }}
        a {{ color: #0969da; text-decoration: none; }}
        a:hover {{ text-decoration: underline; }}
    </style>
</head>
<body>
    <h1 class="header">astgrep API Documentation</h1>
    <p>REST API for advanced static code analysis and security vulnerability detection.</p>

    <h2>Analysis Endpoints</h2>

    <div class="endpoint">
        <span class="method">POST</span> <span class="path">/api/v1/analyze</span>
        <p>Analyze a code snippet directly.</p>
    </div>

    <div class="endpoint">
        <span class="method">POST</span> <span class="path">/api/v1/analyze/file</span>
        <p>Analyze an uploaded file.</p>
    </div>

    <div class="endpoint">
        <span class="method">GET</span> <span class="path">/api/v1/health</span>
        <p>Health check endpoint.</p>
    </div>

    <div class="endpoint">
        <span class="method">GET</span> <span class="path">/api/v1/version</span>
        <p>Get API version information.</p>
    </div>

    <h2>Documentation</h2>
    <div class="endpoint">
        <span class="method">DOCS</span> <span class="path"><a href="/docs/guide">/docs/guide</a></span>
        <p>Rule Writing Guide rendered from docs/astgrep-Guide.md (includes Embedded SQL preprocessor).</p>
        <p><a href="/docs/guide#嵌入式-sql-预处理器">预处理器语法（直达）</a></p>
        <p>Also see: <a href="https://github.com/c2j/astgrep/blob/main/docs/astgrep-Guide.md" target="_blank" rel="noreferrer">Open on GitHub</a></p>
    </div>

    <footer style="margin-top: 40px; color: #6c757d;">
        <p>astgrep API v{} | <a href="/">Back to API Root</a></p>
    </footer>
</body>
</html>"#,
        version
    );

    Ok(Html(html))
}

/// Render docs/astgrep-Guide.md as HTML at /docs/guide
pub async fn rule_guide() -> WebResult<Html<String>> {
    // Embed the guide at compile-time; path relative to this crate
    let md = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../docs/astgrep-Guide.md"));
    let md_json = serde_json::to_string(&md).unwrap_or_else(|_| "\"Failed to load guide.\"".to_string());

    let html = format!(
        r#"<!DOCTYPE html>
<html>
<head>
  <meta charset="utf-8"/>
  <title>astgrep Rule Guide</title>
  <style>
    body {{ font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Arial, sans-serif; margin: 24px; }}
    .container {{ max-width: 980px; margin: 0 auto; }}
    .markdown-body {{ line-height: 1.6; }}
    pre, code {{ font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, "Liberation Mono", "Courier New", monospace; }}
    pre {{ background: #f6f8fa; padding: 12px; overflow: auto; border-radius: 6px; }}
    code {{ background: #f6f8fa; padding: 2px 4px; border-radius: 4px; }}
    a {{ color: #0969da; text-decoration: none; }}
    a:hover {{ text-decoration: underline; }}
    .topnav {{ margin-bottom: 16px; }}
  </style>
  <script src="https://cdn.jsdelivr.net/npm/marked/marked.min.js"></script>
</head>
<body>
  <div class="container">
    <div class="topnav">
      <a href="/docs">← Back to API docs</a> ·
      <a href="https://github.com/c2j/astgrep/blob/main/docs/astgrep-Guide.md" target="_blank" rel="noreferrer">Open on GitHub</a>
    </div>
    <div id="content" class="markdown-body">Loading guide…</div>
  </div>
  <script>
    const md = JSON.parse({md});
    document.getElementById('content').innerHTML = window.marked.parse(md);
  </script>
</body>
</html>"#,
        md = md_json,
    );

    Ok(Html(html))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_api_docs() {
        let result = api_docs().await;
        assert!(result.is_ok());

        let html = result.unwrap().0;
        assert!(html.contains("astgrep API Documentation"));
        assert!(html.contains("/api/v1/analyze"));
        assert!(html.contains("/api/v1/health"));
        assert!(html.contains("/docs/guide"));
    }

    #[tokio::test]
    async fn test_rule_guide() {
        let result = rule_guide().await;
        assert!(result.is_ok());
        let html = result.unwrap().0;
        assert!(html.contains("astgrep Rule Guide"));
        assert!(html.contains("marked.min.js"));
    }
}
