use crate::constant_propagation::{ConstantPropagator, ConstantValue};
use astgrep_core::{AstNode, Language};
use astgrep_parser::TreeSitterParser;

fn main() {
    println!("Debugging constant propagation...");

    let source = r#"
function example() {
    const x = 5;
    return x;
}
"#;

    let parser = TreeSitterParser::new(Language::JavaScript);
    let ast = parser.parse(source).unwrap();

    let mut propagator = ConstantPropagator::new();
    let constants = propagator.analyze_ast(&ast).unwrap();

    println!("Constants: {:?}", constants);
}
