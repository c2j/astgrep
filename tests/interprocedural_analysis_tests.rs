//! Tests for inter-procedural data flow analysis
//!
//! Tests for cross-function taint tracking, call graph construction,
//! and symbol propagation.

use cr_dataflow::{CallGraph, FunctionSignature, InterproceduralTaintTracker, SymbolPropagator};

#[test]
fn test_call_graph_creation() {
    let mut graph = CallGraph::new();
    
    let sig_main = FunctionSignature {
        name: "main".to_string(),
        param_count: 0,
        language: "java".to_string(),
    };
    
    let main_id = graph.add_function(sig_main, vec![], None, 0);
    assert_eq!(main_id.0, 0);
}

#[test]
fn test_call_graph_add_multiple_functions() {
    let mut graph = CallGraph::new();
    
    let sig1 = FunctionSignature {
        name: "foo".to_string(),
        param_count: 1,
        language: "java".to_string(),
    };
    
    let sig2 = FunctionSignature {
        name: "bar".to_string(),
        param_count: 2,
        language: "java".to_string(),
    };
    
    let id1 = graph.add_function(sig1, vec!["x".to_string()], None, 0);
    let id2 = graph.add_function(sig2, vec!["a".to_string(), "b".to_string()], None, 1);
    
    assert_eq!(id1.0, 0);
    assert_eq!(id2.0, 1);
    assert_eq!(graph.functions().len(), 2);
}

#[test]
fn test_call_graph_add_call() {
    let mut graph = CallGraph::new();
    
    let sig_main = FunctionSignature {
        name: "main".to_string(),
        param_count: 0,
        language: "java".to_string(),
    };
    
    let sig_foo = FunctionSignature {
        name: "foo".to_string(),
        param_count: 1,
        language: "java".to_string(),
    };
    
    let main_id = graph.add_function(sig_main, vec![], None, 0);
    graph.add_function(sig_foo.clone(), vec!["x".to_string()], None, 1);
    
    let call_id = graph.add_call(main_id, sig_foo, vec!["42".to_string()], 2);
    assert_eq!(call_id, 0);
    assert!(graph.calls_from(main_id).is_some());
}

#[test]
fn test_call_graph_find_callers() {
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
    
    let main_id = graph.add_function(sig_main, vec![], None, 0);
    graph.add_function(sig_foo.clone(), vec![], None, 1);
    
    graph.add_call(main_id, sig_foo.clone(), vec![], 2);
    
    let callers = graph.find_callers(&sig_foo);
    assert_eq!(callers.len(), 1);
    assert_eq!(callers[0], main_id);
}

#[test]
fn test_call_graph_find_callees() {
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
    
    let main_id = graph.add_function(sig_main, vec![], None, 0);
    graph.add_function(sig_foo.clone(), vec![], None, 1);
    graph.add_function(sig_bar.clone(), vec![], None, 2);
    
    graph.add_call(main_id, sig_foo.clone(), vec![], 3);
    graph.add_call(main_id, sig_bar.clone(), vec![], 4);
    
    let callees = graph.find_callees(main_id);
    assert_eq!(callees.len(), 2);
}

#[test]
fn test_call_graph_trace_path() {
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
    
    let main_id = graph.add_function(sig_main, vec![], None, 0);
    let foo_id = graph.add_function(sig_foo.clone(), vec![], None, 1);
    let bar_id = graph.add_function(sig_bar.clone(), vec![], None, 2);
    
    graph.add_call(main_id, sig_foo.clone(), vec![], 3);
    graph.add_call(foo_id, sig_bar.clone(), vec![], 4);
    
    let path = graph.trace_path(main_id, bar_id);
    assert!(path.is_some());
    let path = path.unwrap();
    assert_eq!(path.len(), 3);
    assert_eq!(path[0], main_id);
    assert_eq!(path[1], foo_id);
    assert_eq!(path[2], bar_id);
}

#[test]
fn test_call_graph_has_call_path() {
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
    
    let main_id = graph.add_function(sig_main, vec![], None, 0);
    let foo_id = graph.add_function(sig_foo.clone(), vec![], None, 1);
    
    graph.add_call(main_id, sig_foo, vec![], 2);
    
    assert!(graph.has_call_path(main_id, foo_id));
    assert!(!graph.has_call_path(foo_id, main_id));
}

#[test]
fn test_call_graph_reachable_functions() {
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
    
    let main_id = graph.add_function(sig_main, vec![], None, 0);
    let foo_id = graph.add_function(sig_foo.clone(), vec![], None, 1);
    let bar_id = graph.add_function(sig_bar.clone(), vec![], None, 2);
    
    graph.add_call(main_id, sig_foo.clone(), vec![], 3);
    graph.add_call(foo_id, sig_bar.clone(), vec![], 4);
    
    let reachable = graph.reachable_functions(main_id);
    assert!(reachable.contains(&main_id));
    assert!(reachable.contains(&foo_id));
    assert!(reachable.contains(&bar_id));
    assert_eq!(reachable.len(), 3);
}

#[test]
fn test_symbol_propagator_define_and_use() {
    let mut propagator = SymbolPropagator::new();
    
    propagator.define_symbol("x".to_string(), 0, "int".to_string());
    propagator.use_symbol("x".to_string(), 1);
    propagator.use_symbol("x".to_string(), 2);
    
    assert_eq!(propagator.get_definition("x"), Some(0));
    assert_eq!(propagator.get_type("x"), Some("int"));
    
    let uses = propagator.get_uses("x").unwrap();
    assert_eq!(uses.len(), 2);
    assert_eq!(uses[0], 1);
    assert_eq!(uses[1], 2);
}

#[test]
fn test_symbol_propagator_multiple_symbols() {
    let mut propagator = SymbolPropagator::new();
    
    propagator.define_symbol("x".to_string(), 0, "int".to_string());
    propagator.define_symbol("y".to_string(), 1, "string".to_string());
    propagator.define_symbol("z".to_string(), 2, "bool".to_string());
    
    assert_eq!(propagator.get_type("x"), Some("int"));
    assert_eq!(propagator.get_type("y"), Some("string"));
    assert_eq!(propagator.get_type("z"), Some("bool"));
}

#[test]
fn test_symbol_propagator_clear() {
    let mut propagator = SymbolPropagator::new();
    
    propagator.define_symbol("x".to_string(), 0, "int".to_string());
    propagator.use_symbol("x".to_string(), 1);
    
    assert!(propagator.get_definition("x").is_some());
    
    propagator.clear();
    
    assert!(propagator.get_definition("x").is_none());
    assert!(propagator.get_uses("x").is_none());
}

#[test]
fn test_interprocedural_tracker_creation() {
    let call_graph = CallGraph::new();
    let tracker = InterproceduralTaintTracker::new(call_graph);
    
    // Verify tracker is created successfully
    assert!(tracker.entry_taints(cr_dataflow::FunctionId(0)).is_none());
}

#[test]
fn test_call_graph_parameter_mapping() {
    let mut graph = CallGraph::new();
    
    let sig_main = FunctionSignature {
        name: "main".to_string(),
        param_count: 0,
        language: "java".to_string(),
    };
    
    let sig_foo = FunctionSignature {
        name: "foo".to_string(),
        param_count: 2,
        language: "java".to_string(),
    };
    
    let main_id = graph.add_function(sig_main, vec![], None, 0);
    graph.add_function(sig_foo.clone(), vec!["x".to_string(), "y".to_string()], None, 1);
    
    let call_id = graph.add_call(main_id, sig_foo, vec!["arg1".to_string(), "arg2".to_string()], 2);
    
    let mapping = graph.get_param_mapping(call_id);
    assert!(mapping.is_some());
    
    let mapping = mapping.unwrap();
    assert_eq!(mapping.mappings.get(&0), Some(&"arg1".to_string()));
    assert_eq!(mapping.mappings.get(&1), Some(&"arg2".to_string()));
}

#[test]
fn test_call_graph_no_path_between_unrelated_functions() {
    let mut graph = CallGraph::new();
    
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
    
    let foo_id = graph.add_function(sig_foo, vec![], None, 0);
    let bar_id = graph.add_function(sig_bar, vec![], None, 1);
    
    // No calls added, so no path should exist
    assert!(!graph.has_call_path(foo_id, bar_id));
    assert!(!graph.has_call_path(bar_id, foo_id));
}

