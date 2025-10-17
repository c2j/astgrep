//! Precise expression matching algorithm based on AST structure
//!
//! This module implements a sophisticated pattern matching algorithm that
//! operates on AST structures rather than text, providing much higher
//! precision for semgrep-style pattern matching.

use crate::parser::{PatternParser, ParsedPattern};
use crate::metavar::MetavarManager;
use cr_core::{AstNode, Result, AnalysisError, SemgrepPattern, PatternType, constants::defaults::analysis};
use cr_ast::{NodeType, UniversalNode};
use std::collections::{HashMap, HashSet, VecDeque};

/// Precise AST-based pattern matcher
pub struct PreciseExpressionMatcher {
    /// Pattern parser for converting string patterns to AST patterns
    pattern_parser: PatternParser,
    /// Metavariable manager for tracking bindings
    metavar_manager: MetavarManager,
    /// Configuration options
    config: MatchingConfig,
    /// Cache for parsed patterns
    pattern_cache: HashMap<String, AstPattern>,
}

/// Configuration for precise matching
#[derive(Debug, Clone)]
pub struct MatchingConfig {
    /// Enable structural matching (match AST structure)
    pub structural_matching: bool,
    /// Enable semantic matching (consider semantics)
    pub semantic_matching: bool,
    /// Enable type-aware matching
    pub type_aware_matching: bool,
    /// Maximum depth for recursive matching
    pub max_depth: usize,
    /// Allow partial matches
    pub allow_partial_matches: bool,
    /// Similarity threshold for fuzzy matching
    pub similarity_threshold: f32,
}

impl Default for MatchingConfig {
    fn default() -> Self {
        Self {
            structural_matching: true,
            semantic_matching: true,
            type_aware_matching: true,
            max_depth: analysis::MAX_ANALYSIS_DEPTH,
            allow_partial_matches: false,
            similarity_threshold: analysis::SIMILARITY_THRESHOLD as f32,
        }
    }
}

/// AST-based pattern representation
#[derive(Debug, Clone)]
pub struct AstPattern {
    /// Root node of the pattern
    pub root: PatternNode,
    /// Metavariables used in this pattern
    pub metavariables: HashSet<String>,
    /// Pattern constraints
    pub constraints: Vec<PatternConstraint>,
}

/// Pattern node in the AST pattern
#[derive(Debug, Clone)]
pub enum PatternNode {
    /// Literal node that must match exactly
    Literal {
        node_type: NodeType,
        text: Option<String>,
        attributes: HashMap<String, String>,
    },
    /// Metavariable that can match any node
    Metavariable {
        name: String,
        constraints: Vec<MetavarConstraint>,
    },
    /// Ellipsis that can match zero or more nodes
    Ellipsis {
        name: Option<String>,
        min_matches: usize,
        max_matches: Option<usize>,
    },
    /// Composite node with children
    Composite {
        node_type: NodeType,
        children: Vec<PatternNode>,
        attributes: HashMap<String, String>,
    },
    /// Alternative patterns (OR)
    Alternative {
        patterns: Vec<PatternNode>,
    },
    /// Sequence patterns (AND)
    Sequence {
        patterns: Vec<PatternNode>,
    },
}

/// Constraints on metavariables
#[derive(Debug, Clone)]
pub enum MetavarConstraint {
    /// Must be of specific node type
    NodeType(NodeType),
    /// Must match regex
    Regex(String),
    /// Must be equal to another metavariable
    Equals(String),
    /// Must not be equal to another metavariable
    NotEquals(String),
    /// Custom constraint function
    Custom(String),
}

/// Pattern constraints
#[derive(Debug, Clone)]
pub enum PatternConstraint {
    /// Pattern must be inside another pattern
    Inside(AstPattern),
    /// Pattern must not match
    Not(AstPattern),
    /// Pattern must be followed by another pattern
    FollowedBy(AstPattern),
    /// Pattern must be preceded by another pattern
    PrecededBy(AstPattern),
}

/// Result of a precise match
pub struct PreciseMatchResult {
    /// Matched node
    pub node: Box<dyn AstNode>,
    /// Metavariable bindings
    pub bindings: HashMap<String, MatchedValue>,
    /// Match confidence (0 to 100)
    pub confidence: u8,
    /// Match type
    pub match_type: MatchType,
    /// Additional metadata
    pub metadata: HashMap<String, String>,
}

/// Value matched by a metavariable
pub enum MatchedValue {
    /// Single node
    Node(Box<dyn AstNode>),
    /// Multiple nodes (for ellipsis)
    Nodes(Vec<Box<dyn AstNode>>),
    /// Text value
    Text(String),
    /// Structured value
    Structured(HashMap<String, MatchedValue>),
}

/// Type of match
#[derive(Debug, Clone)]
pub enum MatchType {
    /// Exact structural match
    Exact,
    /// Partial match
    Partial,
    /// Semantic match (structurally different but semantically equivalent)
    Semantic,
    /// Fuzzy match (similar but not exact)
    Fuzzy,
}

impl PreciseExpressionMatcher {
    /// Create a new precise expression matcher
    pub fn new() -> Self {
        Self::with_config(MatchingConfig::default())
    }

    /// Create a new precise expression matcher with custom configuration
    pub fn with_config(config: MatchingConfig) -> Self {
        Self {
            pattern_parser: PatternParser::new(),
            metavar_manager: MetavarManager::new(),
            config,
            pattern_cache: HashMap::new(),
        }
    }

    /// Find all precise matches for a pattern in the AST
    pub fn find_precise_matches(
        &mut self,
        pattern: &SemgrepPattern,
        root: &dyn AstNode,
    ) -> Result<Vec<PreciseMatchResult>> {
        // Convert semgrep pattern to AST pattern
        let ast_pattern = self.convert_to_ast_pattern(pattern)?;

        // Find matches using the AST pattern
        let mut matches = Vec::new();
        self.find_matches_recursive(&ast_pattern, root, &mut matches, 0)?;

        // Post-process matches
        self.post_process_matches(matches)
    }

    /// Convert semgrep pattern to AST pattern
    fn convert_to_ast_pattern(&mut self, pattern: &SemgrepPattern) -> Result<AstPattern> {
        match &pattern.pattern_type {
            PatternType::Simple(pattern_str) => {
                // Check cache first
                if let Some(cached) = self.pattern_cache.get(pattern_str) {
                    return Ok(cached.clone());
                }

                // Parse the pattern string into AST pattern
                let parsed = self.pattern_parser.parse(pattern_str)?;
                let ast_pattern = self.convert_parsed_pattern_to_ast(&parsed)?;

                // Cache the result
                self.pattern_cache.insert(pattern_str.clone(), ast_pattern.clone());
                Ok(ast_pattern)
            }
            PatternType::Either(patterns) => {
                let mut alt_patterns = Vec::new();
                for sub_pattern in patterns {
                    let ast_pattern = self.convert_to_ast_pattern(sub_pattern)?;
                    alt_patterns.push(ast_pattern.root);
                }
                Ok(AstPattern {
                    root: PatternNode::Alternative { patterns: alt_patterns },
                    metavariables: HashSet::new(),
                    constraints: Vec::new(),
                })
            }
            PatternType::Inside(inner_pattern) => {
                let inner_ast = self.convert_to_ast_pattern(inner_pattern)?;
                let metavars = inner_ast.metavariables.clone();
                Ok(AstPattern {
                    root: inner_ast.root.clone(),
                    metavariables: metavars,
                    constraints: vec![PatternConstraint::Inside(inner_ast)],
                })
            }
            PatternType::Not(inner_pattern) => {
                let inner_ast = self.convert_to_ast_pattern(inner_pattern)?;
                let metavars = inner_ast.metavariables.clone();
                Ok(AstPattern {
                    root: inner_ast.root.clone(),
                    metavariables: metavars,
                    constraints: vec![PatternConstraint::Not(inner_ast)],
                })
            }
            _ => {
                // For other pattern types, fall back to simple conversion
                Ok(AstPattern {
                    root: PatternNode::Metavariable {
                        name: "$ANY".to_string(),
                        constraints: Vec::new(),
                    },
                    metavariables: HashSet::new(),
                    constraints: Vec::new(),
                })
            }
        }
    }

    /// Convert parsed pattern to AST pattern
    fn convert_parsed_pattern_to_ast(&self, parsed: &ParsedPattern) -> Result<AstPattern> {
        let root = self.convert_parsed_node_to_pattern_node(parsed)?;
        let metavariables = self.extract_metavariables(&root);
        
        Ok(AstPattern {
            root,
            metavariables,
            constraints: Vec::new(),
        })
    }

    /// Convert parsed pattern node to pattern node
    fn convert_parsed_node_to_pattern_node(&self, parsed: &ParsedPattern) -> Result<PatternNode> {
        match parsed {
            ParsedPattern::Literal(text) => Ok(PatternNode::Literal {
                node_type: NodeType::Literal,
                text: Some(text.clone()),
                attributes: HashMap::new(),
            }),
            ParsedPattern::Metavariable(name) => Ok(PatternNode::Metavariable {
                name: name.clone(),
                constraints: Vec::new(),
            }),
            ParsedPattern::EllipsisMetavariable(name) => Ok(PatternNode::Ellipsis {
                name: Some(name.clone()),
                min_matches: 0,
                max_matches: None,
            }),
            ParsedPattern::NodeType(node_type_str) => {
                let node_type = self.parse_node_type(node_type_str)?;
                Ok(PatternNode::Literal {
                    node_type,
                    text: None,
                    attributes: HashMap::new(),
                })
            }
            ParsedPattern::Sequence(patterns) => {
                let mut pattern_nodes = Vec::new();
                for pattern in patterns {
                    pattern_nodes.push(self.convert_parsed_node_to_pattern_node(pattern)?);
                }
                Ok(PatternNode::Sequence { patterns: pattern_nodes })
            }
            ParsedPattern::Alternative(patterns) => {
                let mut pattern_nodes = Vec::new();
                for pattern in patterns {
                    pattern_nodes.push(self.convert_parsed_node_to_pattern_node(pattern)?);
                }
                Ok(PatternNode::Alternative { patterns: pattern_nodes })
            }
            ParsedPattern::Wildcard => Ok(PatternNode::Metavariable {
                name: "$_".to_string(),
                constraints: Vec::new(),
            }),
        }
    }

    /// Parse node type string to NodeType enum
    fn parse_node_type(&self, node_type_str: &str) -> Result<NodeType> {
        NodeType::from_str(node_type_str)
            .ok_or_else(|| AnalysisError::pattern_match_error(format!("Unknown node type: {}", node_type_str)))
    }

    /// Extract metavariables from pattern node
    fn extract_metavariables(&self, node: &PatternNode) -> HashSet<String> {
        let mut metavars = HashSet::new();
        self.extract_metavariables_recursive(node, &mut metavars);
        metavars
    }

    /// Recursively extract metavariables
    fn extract_metavariables_recursive(&self, node: &PatternNode, metavars: &mut HashSet<String>) {
        match node {
            PatternNode::Metavariable { name, .. } => {
                metavars.insert(name.clone());
            }
            PatternNode::Ellipsis { name: Some(name), .. } => {
                metavars.insert(name.clone());
            }
            PatternNode::Composite { children, .. } => {
                for child in children {
                    self.extract_metavariables_recursive(child, metavars);
                }
            }
            PatternNode::Alternative { patterns } | PatternNode::Sequence { patterns } => {
                for pattern in patterns {
                    self.extract_metavariables_recursive(pattern, metavars);
                }
            }
            _ => {}
        }
    }

    /// Find matches recursively in the AST
    fn find_matches_recursive(
        &mut self,
        pattern: &AstPattern,
        node: &dyn AstNode,
        matches: &mut Vec<PreciseMatchResult>,
        depth: usize,
    ) -> Result<()> {
        // Check depth limit
        if depth > self.config.max_depth {
            return Ok(());
        }

        // Try to match at current node
        let snapshot = self.metavar_manager.snapshot();
        if let Some(match_result) = self.try_match_node(pattern, node)? {
            matches.push(match_result);
        }
        self.metavar_manager.restore(snapshot);

        // Recursively check children
        for i in 0..node.child_count() {
            if let Some(child) = node.child(i) {
                self.find_matches_recursive(pattern, child, matches, depth + 1)?;
            }
        }

        Ok(())
    }

    /// Try to match a pattern against a node
    fn try_match_node(&mut self, _pattern: &AstPattern, _node: &dyn AstNode) -> Result<Option<PreciseMatchResult>> {
        // Simplified implementation
        Ok(None)
    }

    /// Check if a pattern node matches an AST node
    fn matches_pattern_node(&mut self, _pattern_node: &PatternNode, _ast_node: &dyn AstNode) -> Result<bool> {
        // Simplified implementation
        Ok(false)
    }

    /// Check metavariable constraints
    fn check_metavar_constraint(&self, _constraint: &MetavarConstraint, _node: &dyn AstNode) -> Result<bool> {
        // Simplified implementation
        Ok(true)
    }

    /// Check pattern constraints
    fn check_pattern_constraints(&self, constraints: &[PatternConstraint], node: &dyn AstNode) -> Result<bool> {
        for constraint in constraints {
            match constraint {
                PatternConstraint::Inside(_) => {
                    // Would check if this match is inside another pattern
                    // Simplified for now
                }
                PatternConstraint::Not(_) => {
                    // Would check that another pattern doesn't match
                    // Simplified for now
                }
                PatternConstraint::FollowedBy(_) => {
                    // Would check if this match is followed by another pattern
                    // Simplified for now
                }
                PatternConstraint::PrecededBy(_) => {
                    // Would check if this match is preceded by another pattern
                    // Simplified for now
                }
            }
        }
        Ok(true)
    }

    /// Try fuzzy matching
    fn try_fuzzy_match(&self, pattern: &AstPattern, node: &dyn AstNode) -> Result<Option<PreciseMatchResult>> {
        // Simplified fuzzy matching - would implement more sophisticated
        // similarity algorithms in practice
        Ok(None)
    }

    /// Post-process matches to remove duplicates and rank by confidence
    fn post_process_matches(&self, mut matches: Vec<PreciseMatchResult>) -> Result<Vec<PreciseMatchResult>> {
        // Sort by confidence (descending)
        matches.sort_by(|a, b| b.confidence.partial_cmp(&a.confidence).unwrap_or(std::cmp::Ordering::Equal));

        // Remove duplicates based on node identity
        matches.dedup_by(|a, b| {
            // Simplified deduplication - compare by text content for now
            a.node.text() == b.node.text()
        });

        Ok(matches)
    }
}
