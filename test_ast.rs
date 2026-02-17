use astgrep_parser::{Language, LanguageParserRegistry};

fn main() {
    let source = r#"public class Class {
  private int x = 5;
  public int y = 5;  
  private int z = 5;


  public void foo() {
    return x;
  }

  public void bar() {
    return y; 
  }

  public void qux() {
    z = 3;
    return z; 
  }

  public void foo1() {
    return this.x;
  }

  public void bar1() {
    return this.y;
  }

  public void qux1() {
    this.z = 3;
    return this.z;
  }
}"#;

    let registry = LanguageParserRegistry::new();
    if let Some(parser) = registry.get_parser(Language::Java) {
        let ast = parser
            .parse(source, std::path::Path::new("test.java"))
            .unwrap();
        print_node(&*ast, 0);
    } else {
        eprintln!("Java parser not found");
    }
}

fn print_node(node: &dyn astgrep_core::AstNode, depth: usize) {
    let indent = "  ".repeat(depth);
    println!("{}Node type: {}", indent, node.node_type());
    if let Some(text) = node.text() {
        println!("{}Text: {:?}", indent, text);
    }
    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            print_node(&*child, depth + 1);
        }
    }
}
