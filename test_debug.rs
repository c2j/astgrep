fn main() {
    let pattern_str = "return 5;";
    println!("Testing pattern: '{}'", pattern_str);

    // Test the current has_literal logic
    let has_literal = pattern_str.trim().split_whitespace().any(|tok| {
        println!("Token: '{}'", tok);
        // Try to parse as i64 after trimming punctuation
        let cleaned = tok.trim().trim_end_matches(';').trim_end_matches(',');
        println!("Cleaned: '{}'", cleaned);
        let parse_result = cleaned.parse::<i64>();
        println!("Parse result: {:?}", parse_result);
        let is_number = parse_result.is_ok();
        let is_string = (tok.starts_with('"') && tok.ends_with('"'))
            || (tok.starts_with('\'') && tok.ends_with('\''));
        println!("Is number: {}, Is string: {}", is_number, is_string);
        is_number || is_string
    });

    println!("\nhas_literal: {}", has_literal);
}
