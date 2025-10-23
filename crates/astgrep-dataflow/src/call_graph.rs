//! Call graph construction and analysis for inter-procedural data flow analysis
//!
//! This module provides functionality to build and analyze call graphs,
//! enabling cross-function taint tracking and data flow analysis.

use astgrep_core::{AstNode, Result};
use std::collections::{HashMap, HashSet, VecDeque};

/// Unique identifier for a function
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FunctionId(pub usize);

/// Function signature for matching calls to definitions
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FunctionSignature {
    pub name: String,
    pub param_count: usize,
    pub language: String,
}

/// Function definition information
#[derive(Debug, Clone)]
pub struct FunctionDef {
    pub id: FunctionId,
    pub signature: FunctionSignature,
    pub node_id: usize,
    pub parameters: Vec<String>,
    pub return_type: Option<String>,
}

/// Function call information
#[derive(Debug, Clone)]
pub struct FunctionCall {
    pub id: usize,
    pub caller_id: FunctionId,
    pub callee_signature: FunctionSignature,
    pub arguments: Vec<String>,
    pub node_id: usize,
}

/// Parameter mapping for a specific call
#[derive(Debug, Clone)]
pub struct ParameterMapping {
    pub call_id: usize,
    pub mappings: HashMap<usize, String>, // param_index -> argument_expression
}

/// Call graph for inter-procedural analysis
#[derive(Debug, Clone)]
pub struct CallGraph {
    /// Function definitions: signature -> function definition
    functions: HashMap<FunctionSignature, FunctionDef>,
    /// Function calls: caller_id -> list of calls
    calls: HashMap<FunctionId, Vec<FunctionCall>>,
    /// Parameter mappings for each call
    param_mappings: HashMap<usize, ParameterMapping>,
    /// Next function ID
    next_func_id: usize,
    /// Next call ID
    next_call_id: usize,
}

impl CallGraph {
    /// Create a new empty call graph
    pub fn new() -> Self {
        Self {
            functions: HashMap::new(),
            calls: HashMap::new(),
            param_mappings: HashMap::new(),
            next_func_id: 0,
            next_call_id: 0,
        }
    }

    /// Add a function definition to the call graph
    pub fn add_function(&mut self, signature: FunctionSignature, parameters: Vec<String>, return_type: Option<String>, node_id: usize) -> FunctionId {
        let id = FunctionId(self.next_func_id);
        self.next_func_id += 1;

        let func_def = FunctionDef {
            id,
            signature: signature.clone(),
            node_id,
            parameters,
            return_type,
        };

        self.functions.insert(signature, func_def);
        id
    }

    /// Add a function call to the call graph
    pub fn add_call(&mut self, caller_id: FunctionId, callee_signature: FunctionSignature, arguments: Vec<String>, node_id: usize) -> usize {
        let call_id = self.next_call_id;
        self.next_call_id += 1;

        let call = FunctionCall {
            id: call_id,
            caller_id,
            callee_signature: callee_signature.clone(),
            arguments: arguments.clone(),
            node_id,
        };

        self.calls.entry(caller_id).or_insert_with(Vec::new).push(call);

        // Create parameter mapping
        if let Some(func_def) = self.functions.get(&callee_signature) {
            let mut mappings = HashMap::new();
            for (i, arg) in arguments.iter().enumerate() {
                if i < func_def.parameters.len() {
                    mappings.insert(i, arg.clone());
                }
            }
            self.param_mappings.insert(call_id, ParameterMapping {
                call_id,
                mappings,
            });
        }

        call_id
    }

    /// Get all functions in the call graph
    pub fn functions(&self) -> &HashMap<FunctionSignature, FunctionDef> {
        &self.functions
    }

    /// Get all calls from a function
    pub fn calls_from(&self, func_id: FunctionId) -> Option<&Vec<FunctionCall>> {
        self.calls.get(&func_id)
    }

    /// Get parameter mapping for a call
    pub fn get_param_mapping(&self, call_id: usize) -> Option<&ParameterMapping> {
        self.param_mappings.get(&call_id)
    }

    /// Find all callers of a function
    pub fn find_callers(&self, signature: &FunctionSignature) -> Vec<FunctionId> {
        let mut callers = Vec::new();
        for (caller_id, calls) in &self.calls {
            for call in calls {
                if call.callee_signature == *signature {
                    callers.push(*caller_id);
                }
            }
        }
        callers
    }

    /// Find all callees of a function
    pub fn find_callees(&self, func_id: FunctionId) -> Vec<FunctionSignature> {
        let mut callees = Vec::new();
        if let Some(calls) = self.calls.get(&func_id) {
            for call in calls {
                callees.push(call.callee_signature.clone());
            }
        }
        callees
    }

    /// Trace a path from one function to another
    pub fn trace_path(&self, from: FunctionId, to: FunctionId) -> Option<Vec<FunctionId>> {
        if from == to {
            return Some(vec![from]);
        }

        let mut queue = VecDeque::new();
        let mut visited = HashSet::new();
        let mut parent = HashMap::new();

        queue.push_back(from);
        visited.insert(from);

        while let Some(current) = queue.pop_front() {
            if current == to {
                // Reconstruct path
                let mut path = vec![to];
                let mut node = to;
                while let Some(&prev) = parent.get(&node) {
                    path.push(prev);
                    node = prev;
                }
                path.reverse();
                return Some(path);
            }

            if let Some(calls) = self.calls.get(&current) {
                for call in calls {
                    if let Some(func_def) = self.functions.get(&call.callee_signature) {
                        let next_id = func_def.id;
                        if !visited.contains(&next_id) {
                            visited.insert(next_id);
                            parent.insert(next_id, current);
                            queue.push_back(next_id);
                        }
                    }
                }
            }
        }

        None
    }

    /// Check if there's a call path from one function to another
    pub fn has_call_path(&self, from: FunctionId, to: FunctionId) -> bool {
        self.trace_path(from, to).is_some()
    }

    /// Get all reachable functions from a given function
    pub fn reachable_functions(&self, from: FunctionId) -> HashSet<FunctionId> {
        let mut reachable = HashSet::new();
        let mut queue = VecDeque::new();

        queue.push_back(from);
        reachable.insert(from);

        while let Some(current) = queue.pop_front() {
            if let Some(calls) = self.calls.get(&current) {
                for call in calls {
                    if let Some(func_def) = self.functions.get(&call.callee_signature) {
                        let next_id = func_def.id;
                        if !reachable.contains(&next_id) {
                            reachable.insert(next_id);
                            queue.push_back(next_id);
                        }
                    }
                }
            }
        }

        reachable
    }

    /// Clear the call graph
    pub fn clear(&mut self) {
        self.functions.clear();
        self.calls.clear();
        self.param_mappings.clear();
        self.next_func_id = 0;
        self.next_call_id = 0;
    }
}

impl Default for CallGraph {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_function() {
        let mut graph = CallGraph::new();
        let sig = FunctionSignature {
            name: "foo".to_string(),
            param_count: 1,
            language: "java".to_string(),
        };
        let id = graph.add_function(sig.clone(), vec!["x".to_string()], None, 0);
        assert_eq!(id, FunctionId(0));
        assert!(graph.functions().contains_key(&sig));
    }

    #[test]
    fn test_add_call() {
        let mut graph = CallGraph::new();
        let caller_sig = FunctionSignature {
            name: "main".to_string(),
            param_count: 0,
            language: "java".to_string(),
        };
        let callee_sig = FunctionSignature {
            name: "foo".to_string(),
            param_count: 1,
            language: "java".to_string(),
        };

        let caller_id = graph.add_function(caller_sig, vec![], None, 0);
        graph.add_function(callee_sig.clone(), vec!["x".to_string()], None, 1);

        let call_id = graph.add_call(caller_id, callee_sig.clone(), vec!["42".to_string()], 2);
        assert_eq!(call_id, 0);
        assert!(graph.calls_from(caller_id).is_some());
    }

    #[test]
    fn test_trace_path() {
        let mut graph = CallGraph::new();
        let sig_main = FunctionSignature {
            name: "main".to_string(),
            param_count: 0,
            language: "java".to_string(),
        };
        let sig_foo = FunctionSignature {
            name: "foo".to_string(),
            param_count: 0,
            language: "java".to_string(),
        };
        let sig_bar = FunctionSignature {
            name: "bar".to_string(),
            param_count: 0,
            language: "java".to_string(),
        };

        let main_id = graph.add_function(sig_main.clone(), vec![], None, 0);
        let foo_id = graph.add_function(sig_foo.clone(), vec![], None, 1);
        let bar_id = graph.add_function(sig_bar.clone(), vec![], None, 2);

        graph.add_call(main_id, sig_foo.clone(), vec![], 3);
        graph.add_call(foo_id, sig_bar.clone(), vec![], 4);

        let path = graph.trace_path(main_id, bar_id);
        assert!(path.is_some());
        assert_eq!(path.unwrap(), vec![main_id, foo_id, bar_id]);
    }

    #[test]
    fn test_reachable_functions() {
        let mut graph = CallGraph::new();
        let sig_main = FunctionSignature {
            name: "main".to_string(),
            param_count: 0,
            language: "java".to_string(),
        };
        let sig_foo = FunctionSignature {
            name: "foo".to_string(),
            param_count: 0,
            language: "java".to_string(),
        };

        let main_id = graph.add_function(sig_main.clone(), vec![], None, 0);
        let foo_id = graph.add_function(sig_foo.clone(), vec![], None, 1);

        graph.add_call(main_id, sig_foo.clone(), vec![], 2);

        let reachable = graph.reachable_functions(main_id);
        assert!(reachable.contains(&main_id));
        assert!(reachable.contains(&foo_id));
    }
}

