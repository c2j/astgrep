//! Version information handler

use axum::response::Json;
use crate::{models::VersionInfo, WebResult};

/// Get version information
pub async fn get_version() -> WebResult<Json<VersionInfo>> {
    let version_info = VersionInfo {
        version: env!("CARGO_PKG_VERSION").to_string(),
        build_timestamp: option_env!("BUILD_TIMESTAMP")
            .unwrap_or("unknown")
            .to_string(),
        git_commit: option_env!("GIT_COMMIT")
            .unwrap_or("unknown")
            .to_string(),
        rust_version: option_env!("RUST_VERSION")
            .unwrap_or(env!("CARGO_PKG_RUST_VERSION"))
            .to_string(),
        features: get_enabled_features(),
    };

    Ok(Json(version_info))
}

/// Get list of enabled features
fn get_enabled_features() -> Vec<String> {
    let mut features = Vec::new();

    // Core features
    features.push("static-analysis".to_string());
    features.push("multi-language".to_string());
    features.push("rest-api".to_string());

    // Optional features
    #[cfg(feature = "database")]
    features.push("database".to_string());

    #[cfg(feature = "auth")]
    features.push("authentication".to_string());

    #[cfg(feature = "metrics")]
    features.push("metrics".to_string());

    // Language support
    features.push("java".to_string());
    features.push("javascript".to_string());
    features.push("python".to_string());
    features.push("sql".to_string());
    features.push("bash".to_string());

    // Analysis features
    features.push("pattern-matching".to_string());
    features.push("security-scanning".to_string());
    features.push("code-quality".to_string());

    // Output formats
    features.push("json-output".to_string());
    features.push("sarif-output".to_string());
    features.push("xml-output".to_string());

    features.sort();
    features
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_get_version() {
        let result = get_version().await;
        assert!(result.is_ok());

        let version_info = result.unwrap().0;
        assert!(!version_info.version.is_empty());
        assert!(!version_info.features.is_empty());
        assert!(version_info.features.contains(&"static-analysis".to_string()));
    }

    #[test]
    fn test_get_enabled_features() {
        let features = get_enabled_features();
        assert!(!features.is_empty());
        assert!(features.contains(&"static-analysis".to_string()));
        assert!(features.contains(&"multi-language".to_string()));
        assert!(features.contains(&"java".to_string()));
        
        // Features should be sorted
        let mut sorted_features = features.clone();
        sorted_features.sort();
        assert_eq!(features, sorted_features);
    }
}
