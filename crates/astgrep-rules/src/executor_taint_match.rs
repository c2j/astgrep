/// Represents a taint match (source or sink)
struct TaintMatch {
    node: Box<dyn AstNode>,
    bindings: HashMap<String, String>,
    var_name: Option<String>,
}

impl Clone for TaintMatch {
    fn clone(&self) -> Self {
        Self {
            node: self.node.clone_node(),
            bindings: self.bindings.clone(),
            var_name: self.var_name.clone(),
        }
    }
}