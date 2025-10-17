//! Root endpoint handler

use axum::response::Html;
use crate::WebResult;

/// Root endpoint - returns API information
pub async fn root() -> WebResult<Html<&'static str>> {
    let html = r#"
<!DOCTYPE html>
<html>
<head>
    <title>CR-SemService API</title>
    <style>
        body { font-family: Arial, sans-serif; margin: 40px; }
        .header { color: #2c3e50; }
        .endpoint { background: #f8f9fa; padding: 10px; margin: 10px 0; border-left: 4px solid #007bff; }
        .method { font-weight: bold; color: #007bff; }
        .path { font-family: monospace; background: #e9ecef; padding: 2px 6px; }
    </style>
</head>
<body>
    <h1 class="header">🔍 CR-SemService API</h1>
    <p>Welcome to the CR-SemService REST API - Advanced Static Code Analysis</p>
    
    <h2>Available Endpoints</h2>
    
    <div class="endpoint">
        <span class="method">GET</span> <span class="path">/api/v1/health</span>
        <p>Health check and system status</p>
    </div>
    
    <div class="endpoint">
        <span class="method">POST</span> <span class="path">/api/v1/analyze</span>
        <p>Analyze code snippet</p>
    </div>
    
    <div class="endpoint">
        <span class="method">POST</span> <span class="path">/api/v1/analyze/file</span>
        <p>Analyze uploaded file</p>
    </div>
    
    <div class="endpoint">
        <span class="method">POST</span> <span class="path">/api/v1/analyze/archive</span>
        <p>Analyze uploaded archive (zip, tar)</p>
    </div>
    
    <div class="endpoint">
        <span class="method">GET</span> <span class="path">/api/v1/jobs/{id}</span>
        <p>Get analysis job status</p>
    </div>
    
    <div class="endpoint">
        <span class="method">GET</span> <span class="path">/api/v1/rules</span>
        <p>List available analysis rules</p>
    </div>
    
    <div class="endpoint">
        <span class="method">POST</span> <span class="path">/api/v1/rules/validate</span>
        <p>Validate rule definitions</p>
    </div>
    
    <div class="endpoint">
        <span class="method">GET</span> <span class="path">/api/v1/version</span>
        <p>Get API version information</p>
    </div>
    
    <div class="endpoint">
        <span class="method">GET</span> <span class="path">/api/v1/metrics</span>
        <p>Get service metrics (Prometheus format)</p>
    </div>
    
    <h2>Documentation</h2>
    <p><a href="/docs">📖 API Documentation</a></p>
    
    <h2>Features</h2>
    <ul>
        <li>🔍 Multi-language static analysis</li>
        <li>🛡️ Security vulnerability detection</li>
        <li>📊 Code quality assessment</li>
        <li>🚀 High-performance parallel processing</li>
        <li>🔧 Custom rule support</li>
        <li>📈 Detailed metrics and reporting</li>
    </ul>
    
    <footer style="margin-top: 40px; color: #6c757d; font-size: 0.9em;">
        <p>CR-SemService v{version} | Built with Rust & Axum</p>
    </footer>
</body>
</html>
    "#.replace("{version}", env!("CARGO_PKG_VERSION"));

    Ok(Html(Box::leak(html.into_boxed_str())))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_root_endpoint() {
        let result = root().await;
        assert!(result.is_ok());
        
        let html = result.unwrap().0;
        assert!(html.contains("CR-SemService API"));
        assert!(html.contains("/api/v1/health"));
        assert!(html.contains("/api/v1/analyze"));
    }
}
