//! Tests for rule marketplace functionality
//!
//! Tests for verifying that the rule marketplace features work correctly.

use astgrep_rules::marketplace::{MarketplaceRule, RuleMarketplace};

#[test]
fn test_marketplace_rule_new() {
    let rule = MarketplaceRule::new(
        "sql_injection".to_string(),
        "SQL Injection Detection".to_string(),
        "security_team".to_string(),
    );
    
    assert_eq!(rule.id, "sql_injection");
    assert_eq!(rule.name, "SQL Injection Detection");
    assert_eq!(rule.author, "security_team");
    assert_eq!(rule.downloads, 0);
    assert_eq!(rule.rating, 0.0);
    assert_eq!(rule.rating_count, 0);
    assert!(!rule.verified);
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
    
    rule.add_rating(4.0);
    assert_eq!(rule.rating, 4.0);
    assert_eq!(rule.rating_count, 3);
}

#[test]
fn test_marketplace_rule_add_invalid_rating() {
    let mut rule = MarketplaceRule::new(
        "rule1".to_string(),
        "Test Rule".to_string(),
        "author".to_string(),
    );
    
    // Try to add invalid ratings
    rule.add_rating(6.0); // Too high
    rule.add_rating(-1.0); // Too low
    
    // Rating should remain 0
    assert_eq!(rule.rating, 0.0);
    assert_eq!(rule.rating_count, 0);
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
    
    rule.increment_downloads();
    assert_eq!(rule.downloads, 2);
}

#[test]
fn test_marketplace_rule_mark_verified() {
    let mut rule = MarketplaceRule::new(
        "rule1".to_string(),
        "Test Rule".to_string(),
        "author".to_string(),
    );
    
    assert!(!rule.verified);
    
    rule.mark_verified();
    assert!(rule.verified);
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
fn test_marketplace_add_multiple_rules() {
    let mut marketplace = RuleMarketplace::new();
    
    for i in 0..5 {
        let rule = MarketplaceRule::new(
            format!("rule{}", i),
            format!("Test Rule {}", i),
            "author".to_string(),
        );
        marketplace.add_rule(rule);
    }
    
    assert_eq!(marketplace.rule_count(), 5);
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
fn test_marketplace_get_rule_mut() {
    let mut marketplace = RuleMarketplace::new();
    let rule = MarketplaceRule::new(
        "rule1".to_string(),
        "Test Rule".to_string(),
        "author".to_string(),
    );
    
    marketplace.add_rule(rule);
    
    if let Some(rule) = marketplace.get_rule_mut("rule1") {
        rule.increment_downloads();
    }
    
    assert_eq!(marketplace.get_rule("rule1").unwrap().downloads, 1);
}

#[test]
fn test_marketplace_search_by_category() {
    let mut marketplace = RuleMarketplace::new();
    
    let mut rule1 = MarketplaceRule::new(
        "rule1".to_string(),
        "Security Rule".to_string(),
        "author".to_string(),
    );
    rule1.category = "security".to_string();
    
    let mut rule2 = MarketplaceRule::new(
        "rule2".to_string(),
        "Performance Rule".to_string(),
        "author".to_string(),
    );
    rule2.category = "performance".to_string();
    
    marketplace.add_rule(rule1);
    marketplace.add_rule(rule2);
    
    let security_rules = marketplace.search_by_category("security");
    assert_eq!(security_rules.len(), 1);
    assert_eq!(security_rules[0].id, "rule1");
}

#[test]
fn test_marketplace_search_by_tag() {
    let mut marketplace = RuleMarketplace::new();
    
    let mut rule = MarketplaceRule::new(
        "rule1".to_string(),
        "Test Rule".to_string(),
        "author".to_string(),
    );
    rule.tags = vec!["sql".to_string(), "injection".to_string()];
    
    marketplace.add_rule(rule);
    
    let sql_rules = marketplace.search_by_tag("sql");
    assert_eq!(sql_rules.len(), 1);
    
    let injection_rules = marketplace.search_by_tag("injection");
    assert_eq!(injection_rules.len(), 1);
}

#[test]
fn test_marketplace_search_by_name() {
    let mut marketplace = RuleMarketplace::new();
    
    let rule1 = MarketplaceRule::new(
        "rule1".to_string(),
        "SQL Injection Detection".to_string(),
        "author".to_string(),
    );
    
    let rule2 = MarketplaceRule::new(
        "rule2".to_string(),
        "XSS Detection".to_string(),
        "author".to_string(),
    );
    
    marketplace.add_rule(rule1);
    marketplace.add_rule(rule2);
    
    let sql_rules = marketplace.search_by_name("SQL");
    assert_eq!(sql_rules.len(), 1);
    assert_eq!(sql_rules[0].id, "rule1");
}

#[test]
fn test_marketplace_get_top_rated() {
    let mut marketplace = RuleMarketplace::new();
    
    let mut rule1 = MarketplaceRule::new(
        "rule1".to_string(),
        "Rule 1".to_string(),
        "author".to_string(),
    );
    rule1.add_rating(5.0);
    
    let mut rule2 = MarketplaceRule::new(
        "rule2".to_string(),
        "Rule 2".to_string(),
        "author".to_string(),
    );
    rule2.add_rating(3.0);
    
    marketplace.add_rule(rule1);
    marketplace.add_rule(rule2);
    
    let top_rated = marketplace.get_top_rated(1);
    assert_eq!(top_rated.len(), 1);
    assert_eq!(top_rated[0].id, "rule1");
}

#[test]
fn test_marketplace_get_most_downloaded() {
    let mut marketplace = RuleMarketplace::new();
    
    let mut rule1 = MarketplaceRule::new(
        "rule1".to_string(),
        "Rule 1".to_string(),
        "author".to_string(),
    );
    rule1.downloads = 100;
    
    let mut rule2 = MarketplaceRule::new(
        "rule2".to_string(),
        "Rule 2".to_string(),
        "author".to_string(),
    );
    rule2.downloads = 50;
    
    marketplace.add_rule(rule1);
    marketplace.add_rule(rule2);
    
    let most_downloaded = marketplace.get_most_downloaded(1);
    assert_eq!(most_downloaded.len(), 1);
    assert_eq!(most_downloaded[0].id, "rule1");
}

#[test]
fn test_marketplace_get_verified_rules() {
    let mut marketplace = RuleMarketplace::new();
    
    let mut rule1 = MarketplaceRule::new(
        "rule1".to_string(),
        "Rule 1".to_string(),
        "author".to_string(),
    );
    rule1.mark_verified();
    
    let rule2 = MarketplaceRule::new(
        "rule2".to_string(),
        "Rule 2".to_string(),
        "author".to_string(),
    );
    
    marketplace.add_rule(rule1);
    marketplace.add_rule(rule2);
    
    let verified = marketplace.get_verified_rules();
    assert_eq!(verified.len(), 1);
    assert_eq!(verified[0].id, "rule1");
}

#[test]
fn test_marketplace_get_all_rules() {
    let mut marketplace = RuleMarketplace::new();
    
    for i in 0..3 {
        let rule = MarketplaceRule::new(
            format!("rule{}", i),
            format!("Rule {}", i),
            "author".to_string(),
        );
        marketplace.add_rule(rule);
    }
    
    let all_rules = marketplace.get_all_rules();
    assert_eq!(all_rules.len(), 3);
}

#[test]
fn test_marketplace_get_categories() {
    let mut marketplace = RuleMarketplace::new();
    
    let mut rule1 = MarketplaceRule::new(
        "rule1".to_string(),
        "Rule 1".to_string(),
        "author".to_string(),
    );
    rule1.category = "security".to_string();
    
    let mut rule2 = MarketplaceRule::new(
        "rule2".to_string(),
        "Rule 2".to_string(),
        "author".to_string(),
    );
    rule2.category = "performance".to_string();
    
    marketplace.add_rule(rule1);
    marketplace.add_rule(rule2);
    
    let categories = marketplace.get_categories();
    assert_eq!(categories.len(), 2);
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
    
    let removed = marketplace.remove_rule("rule1");
    assert!(removed.is_some());
    assert_eq!(marketplace.rule_count(), 0);
}

#[test]
fn test_marketplace_remove_nonexistent_rule() {
    let mut marketplace = RuleMarketplace::new();
    let removed = marketplace.remove_rule("nonexistent");
    assert!(removed.is_none());
}

#[test]
fn test_marketplace_default() {
    let marketplace = RuleMarketplace::default();
    assert_eq!(marketplace.rule_count(), 0);
}

