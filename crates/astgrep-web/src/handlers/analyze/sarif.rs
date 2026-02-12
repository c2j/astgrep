use crate::models::AnalysisResults;

/// Convert analysis results to SARIF format
pub fn convert_to_sarif(results: &AnalysisResults) -> crate::models::SarifOutput {
    use crate::models::{
        SarifArtifactLocation, SarifLocation, SarifMessage, SarifOutput, SarifPhysicalLocation,
        SarifRegion, SarifResult, SarifRun, SarifTool, SarifToolDriver,
    };

    let results: Vec<SarifResult> = results
        .findings
        .iter()
        .map(|finding| SarifResult {
            rule_id: finding.rule_id.clone(),
            message: SarifMessage {
                text: finding.message.clone(),
            },
            locations: vec![SarifLocation {
                physical_location: SarifPhysicalLocation {
                    artifact_location: SarifArtifactLocation {
                        uri: finding.location.file.clone(),
                    },
                    region: SarifRegion {
                        start_line: finding.location.start_line as i64,
                        start_column: finding.location.start_column as i64,
                        end_line: finding.location.end_line as i64,
                        end_column: finding.location.end_column as i64,
                    },
                },
            }],
            level: match finding.severity.as_str() {
                "critical" | "error" => "error".to_string(),
                "warning" => "warning".to_string(),
                _ => "note".to_string(),
            },
        })
        .collect();

    SarifOutput {
        version: "2.1.0".to_string(),
        schema: "https://json.schemastore.org/sarif-2.1.0.json".to_string(),
        runs: vec![SarifRun {
            tool: SarifTool {
                driver: SarifToolDriver {
                    name: "astgrep".to_string(),
                    version: Some(env!("CARGO_PKG_VERSION").to_string()),
                    information_uri: Some("https://github.com/ast-grep/ast-grep".to_string()),
                    rules: None,
                },
            },
            results,
        }],
    }
}
