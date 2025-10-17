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
    <title>CR-SemService API Documentation</title>
    <style>
        body {{ font-family: Arial, sans-serif; margin: 40px; line-height: 1.6; }}
        .header {{ color: #2c3e50; }}
        .endpoint {{ background: #f8f9fa; padding: 15px; margin: 15px 0; }}
        .method {{ font-weight: bold; color: #007bff; }}
        .path {{ font-family: monospace; background: #e9ecef; padding: 2px 6px; }}
    </style>
</head>
<body>
    <h1 class="header">CR-SemService API Documentation</h1>
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
    
    <footer style="margin-top: 40px; color: #6c757d;">
        <p>CR-SemService API v{} | <a href="/">Back to API Root</a></p>
    </footer>
</body>
</html>"#,
        version
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
        assert!(html.contains("CR-SemService API Documentation"));
        assert!(html.contains("/api/v1/analyze"));
        assert!(html.contains("/api/v1/health"));
    }
}
