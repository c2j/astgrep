use std::future::Future;
use std::path::PathBuf;

use rmcp::handler::server::tool::{Parameters, ToolRouter};
use rmcp::model::*;
use rmcp::transport::stdio;
use rmcp::{tool, tool_handler, tool_router, Error as McpError, ServerHandler, ServiceExt};

use crate::tools;

#[derive(Clone)]
pub struct AstgrepServer {
    pub rules_dir: Option<PathBuf>,
    tool_router: ToolRouter<Self>,
}

#[tool_router]
impl AstgrepServer {
    pub fn new(rules_dir: Option<PathBuf>) -> Self {
        Self {
            rules_dir,
            tool_router: Self::tool_router(),
        }
    }

    #[tool(description = "Analyze source code for security vulnerabilities and quality issues")]
    async fn analyze_code(
        &self,
        Parameters(req): Parameters<tools::analyze::AnalyzeCodeRequest>,
    ) -> Result<CallToolResult, McpError> {
        match tools::analyze::handle_analyze(req, self.rules_dir.as_deref()).await {
            Ok(result) => {
                let json = serde_json::to_string_pretty(&result)
                    .unwrap_or_else(|e| format!("{{\"error\": \"serialization failed: {e}\"}}"));
                Ok(CallToolResult::success(vec![Content::text(json)]))
            }
            Err(e) => Ok(CallToolResult::error(vec![Content::text(format!(
                "Analysis failed: {}",
                crate::error::analysis_error_to_msg(&e)
            ))])),
        }
    }

    #[tool(description = "Validate a YAML rule file for syntax and semantic correctness")]
    async fn validate_rules(
        &self,
        Parameters(req): Parameters<tools::validate::ValidateRulesRequest>,
    ) -> Result<CallToolResult, McpError> {
        match tools::validate::handle_validate(req).await {
            Ok(results) => {
                let json = serde_json::to_string_pretty(&results)
                    .unwrap_or_else(|e| format!("{{\"error\": \"serialization failed: {e}\"}}"));
                Ok(CallToolResult::success(vec![Content::text(json)]))
            }
            Err(e) => Ok(CallToolResult::error(vec![Content::text(format!(
                "Validation failed: {}",
                crate::error::analysis_error_to_msg(&e)
            ))])),
        }
    }

    #[tool(description = "List all available rules from the configured rules directory")]
    async fn list_rules(&self) -> Result<CallToolResult, McpError> {
        let rules = if let Some(ref dir) = self.rules_dir {
            tools::query::list_rules_from_dir(dir)
        } else {
            Vec::new()
        };
        let json = serde_json::to_string_pretty(&rules)
            .unwrap_or_else(|e| format!("{{\"error\": \"serialization failed: {e}\"}}"));
        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    #[tool(
        description = "List all programming languages supported by astgrep with their file extensions"
    )]
    async fn list_languages(&self) -> Result<CallToolResult, McpError> {
        let languages = tools::query::list_supported_languages();
        let json = serde_json::to_string_pretty(&languages)
            .unwrap_or_else(|e| format!("{{\"error\": \"serialization failed: {e}\"}}"));
        Ok(CallToolResult::success(vec![Content::text(json)]))
    }
}

#[tool_handler]
impl ServerHandler for AstgrepServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            instructions: Some(
                "Static analysis tool for security vulnerabilities and code quality".into(),
            ),
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            ..Default::default()
        }
    }
}

pub async fn serve_stdio(rules_dir: Option<PathBuf>) -> anyhow::Result<()> {
    let server = AstgrepServer::new(rules_dir);
    let service = server.serve(stdio()).await?;
    service.waiting().await?;
    Ok(())
}
