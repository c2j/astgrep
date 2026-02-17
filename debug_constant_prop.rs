use astgrep_core::{AstNode, Language};
use astgrep_dataflow::constant_propagation::{ConstantPropagator, ConstantValue};
use astgrep_parser::TreeSitterParser;

fn main() {
    println!("Debugging constant propagation...");

    let source = r#"
const x = 5;
function example() {
    return x;
}
"#;

    let parser = TreeSitterParser::new(Language::JavaScript);
    let ast = parser.parse(source).unwrap();

    let mut propagator = ConstantPropagator::new();
    let constants = propagator.analyze_ast(&ast).unwrap();

    println!("Found {} constants:", constants.len());
    for (name, value) in &constants {
        println!("  {} = {:?}", name, value);
    }
}
