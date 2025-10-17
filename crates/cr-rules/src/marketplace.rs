//! Rule marketplace for sharing and discovering security rules
//!
//! This module provides functionality for managing a marketplace of security rules,
//! including rule discovery, rating, and community contributions.

use crate::types::Rule;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

/// Represents a rule in the marketplace
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketplaceRule {
    /// Rule ID
    pub id: String,
    /// Rule name
    pub name: String,
    /// Rule description
    pub description: String,
    /// Rule author
    pub author: String,
    /// Rule version
    pub version: String,
    /// Rule category (e.g., "security", "performance", "style")
    pub category: String,
    /// Number of downloads
    pub downloads: u64,
    /// Average rating (0-5)
    pub rating: f32,
    /// Number of ratings
    pub rating_count: u32,
    /// Tags for searching
    pub tags: Vec<String>,
    /// Whether the rule is verified
    pub verified: bool,
    /// Last updated timestamp
    pub last_updated: String,
}

impl MarketplaceRule {
    /// Create a new marketplace rule
    pub fn new(id: String, name: String, author: String) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        Self {
            id,
            name,
            description: String::new(),
            author,
            version: "1.0.0".to_string(),
            category: "security".to_string(),
            downloads: 0,
            rating: 0.0,
            rating_count: 0,
            tags: Vec::new(),
            verified: false,
            last_updated: now.to_string(),
        }
    }

    /// Add a rating to this rule
    pub fn add_rating(&mut self, rating: f32) {
        if rating < 0.0 || rating > 5.0 {
            return;
        }
        
        let total = self.rating * self.rating_count as f32;
        self.rating_count += 1;
        self.rating = (total + rating) / self.rating_count as f32;
    }

    /// Increment download count
    pub fn increment_downloads(&mut self) {
        self.downloads += 1;
    }

    /// Mark as verified
    pub fn mark_verified(&mut self) {
        self.verified = true;
    }
}

/// Rule marketplace
pub struct RuleMarketplace {
    /// Map of rule ID to marketplace rule
    rules: HashMap<String, MarketplaceRule>,
    /// Map of category to rule IDs
    categories: HashMap<String, Vec<String>>,
    /// Map of tag to rule IDs
    tags: HashMap<String, Vec<String>>,
}

impl RuleMarketplace {
    /// Create a new marketplace
    pub fn new() -> Self {
        Self {
            rules: HashMap::new(),
            categories: HashMap::new(),
            tags: HashMap::new(),
        }
    }

    /// Add a rule to the marketplace
    pub fn add_rule(&mut self, rule: MarketplaceRule) {
        let rule_id = rule.id.clone();
        let category = rule.category.clone();

        // Add to rules map
        self.rules.insert(rule_id.clone(), rule.clone());

        // Add to categories map
        self.categories
            .entry(category)
            .or_insert_with(Vec::new)
            .push(rule_id.clone());

        // Add to tags map
        for tag in &rule.tags {
            self.tags
                .entry(tag.clone())
                .or_insert_with(Vec::new)
                .push(rule_id.clone());
        }
    }

    /// Get a rule by ID
    pub fn get_rule(&self, id: &str) -> Option<&MarketplaceRule> {
        self.rules.get(id)
    }

    /// Get a mutable rule by ID
    pub fn get_rule_mut(&mut self, id: &str) -> Option<&mut MarketplaceRule> {
        self.rules.get_mut(id)
    }

    /// Search rules by category
    pub fn search_by_category(&self, category: &str) -> Vec<&MarketplaceRule> {
        self.categories
            .get(category)
            .map(|ids| {
                ids.iter()
                    .filter_map(|id| self.rules.get(id))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Search rules by tag
    pub fn search_by_tag(&self, tag: &str) -> Vec<&MarketplaceRule> {
        self.tags
            .get(tag)
            .map(|ids| {
                ids.iter()
                    .filter_map(|id| self.rules.get(id))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Search rules by name (partial match)
    pub fn search_by_name(&self, name: &str) -> Vec<&MarketplaceRule> {
        let name_lower = name.to_lowercase();
        self.rules
            .values()
            .filter(|rule| rule.name.to_lowercase().contains(&name_lower))
            .collect()
    }

    /// Get top rated rules
    pub fn get_top_rated(&self, limit: usize) -> Vec<&MarketplaceRule> {
        let mut rules: Vec<_> = self.rules.values().collect();
        rules.sort_by(|a, b| b.rating.partial_cmp(&a.rating).unwrap_or(std::cmp::Ordering::Equal));
        rules.into_iter().take(limit).collect()
    }

    /// Get most downloaded rules
    pub fn get_most_downloaded(&self, limit: usize) -> Vec<&MarketplaceRule> {
        let mut rules: Vec<_> = self.rules.values().collect();
        rules.sort_by(|a, b| b.downloads.cmp(&a.downloads));
        rules.into_iter().take(limit).collect()
    }

    /// Get all verified rules
    pub fn get_verified_rules(&self) -> Vec<&MarketplaceRule> {
        self.rules
            .values()
            .filter(|rule| rule.verified)
            .collect()
    }

    /// Get all rules
    pub fn get_all_rules(&self) -> Vec<&MarketplaceRule> {
        self.rules.values().collect()
    }

    /// Get rule count
    pub fn rule_count(&self) -> usize {
        self.rules.len()
    }

    /// Get categories
    pub fn get_categories(&self) -> Vec<&String> {
        self.categories.keys().collect()
    }

    /// Get tags
    pub fn get_tags(&self) -> Vec<&String> {
        self.tags.keys().collect()
    }

    /// Remove a rule
    pub fn remove_rule(&mut self, id: &str) -> Option<MarketplaceRule> {
        if let Some(rule) = self.rules.remove(id) {
            // Remove from categories
            if let Some(ids) = self.categories.get_mut(&rule.category) {
                ids.retain(|rid| rid != id);
            }

            // Remove from tags
            for tag in &rule.tags {
                if let Some(ids) = self.tags.get_mut(tag) {
                    ids.retain(|rid| rid != id);
                }
            }

            Some(rule)
        } else {
            None
        }
    }
}

impl Default for RuleMarketplace {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_marketplace_rule_new() {
        let rule = MarketplaceRule::new(
            "rule1".to_string(),
            "Test Rule".to_string(),
            "author".to_string(),
        );
        assert_eq!(rule.id, "rule1");
        assert_eq!(rule.name, "Test Rule");
        assert_eq!(rule.author, "author");
        assert_eq!(rule.downloads, 0);
        assert_eq!(rule.rating, 0.0);
    }

    #[test]
    fn test_marketplace_rule_add_rating() {
        let mut rule = MarketplaceRule::new(
            "rule1".to_string(),
            "Test Rule".to_string(),
            "author".to_string(),
        );
        
        rule.add_rating(5.0);
        assert_eq!(rule.rating, 5.0);
        assert_eq!(rule.rating_count, 1);
        
        rule.add_rating(3.0);
        assert_eq!(rule.rating, 4.0);
        assert_eq!(rule.rating_count, 2);
    }

    #[test]
    fn test_marketplace_rule_increment_downloads() {
        let mut rule = MarketplaceRule::new(
            "rule1".to_string(),
            "Test Rule".to_string(),
            "author".to_string(),
        );
        
        assert_eq!(rule.downloads, 0);
        rule.increment_downloads();
        assert_eq!(rule.downloads, 1);
    }

    #[test]
    fn test_marketplace_new() {
        let marketplace = RuleMarketplace::new();
        assert_eq!(marketplace.rule_count(), 0);
    }

    #[test]
    fn test_marketplace_add_rule() {
        let mut marketplace = RuleMarketplace::new();
        let rule = MarketplaceRule::new(
            "rule1".to_string(),
            "Test Rule".to_string(),
            "author".to_string(),
        );
        
        marketplace.add_rule(rule);
        assert_eq!(marketplace.rule_count(), 1);
    }

    #[test]
    fn test_marketplace_get_rule() {
        let mut marketplace = RuleMarketplace::new();
        let rule = MarketplaceRule::new(
            "rule1".to_string(),
            "Test Rule".to_string(),
            "author".to_string(),
        );
        
        marketplace.add_rule(rule);
        assert!(marketplace.get_rule("rule1").is_some());
        assert!(marketplace.get_rule("nonexistent").is_none());
    }

    #[test]
    fn test_marketplace_search_by_category() {
        let mut marketplace = RuleMarketplace::new();
        let mut rule = MarketplaceRule::new(
            "rule1".to_string(),
            "Test Rule".to_string(),
            "author".to_string(),
        );
        rule.category = "security".to_string();
        
        marketplace.add_rule(rule);
        let results = marketplace.search_by_category("security");
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_marketplace_search_by_name() {
        let mut marketplace = RuleMarketplace::new();
        let rule = MarketplaceRule::new(
            "rule1".to_string(),
            "SQL Injection".to_string(),
            "author".to_string(),
        );
        
        marketplace.add_rule(rule);
        let results = marketplace.search_by_name("SQL");
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_marketplace_remove_rule() {
        let mut marketplace = RuleMarketplace::new();
        let rule = MarketplaceRule::new(
            "rule1".to_string(),
            "Test Rule".to_string(),
            "author".to_string(),
        );
        
        marketplace.add_rule(rule);
        assert_eq!(marketplace.rule_count(), 1);
        
        marketplace.remove_rule("rule1");
        assert_eq!(marketplace.rule_count(), 0);
    }
}

