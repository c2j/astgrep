#!/usr/bin/env python3
"""
Debug script to understand tree-sitter AST structure
"""

import tree_sitter_python as tspython
import tree_sitter_java as tsjava
import tree_sitter_javascript as tsjs
from tree_sitter import Language, Parser

def print_ast(node, source, indent=0):
    """Print AST structure recursively"""
    node_text = node.text.decode('utf-8') if node.text else ""
    # Limit text display to avoid long output
    if len(node_text) > 50:
        node_text = node_text[:47] + "..."
    
    print("  " * indent + f"{node.type}: '{node_text}' [{node.start_point}-{node.end_point}]")
    
    for child in node.children:
        print_ast(child, source, indent + 1)

def analyze_python_code():
    print("=== PYTHON AST ANALYSIS ===")
    
    # Initialize Python parser
    PY_LANGUAGE = Language(tspython.language(), "python")
    parser = Parser()
    parser.set_language(PY_LANGUAGE)
    
    # Test different Python constructs
    test_cases = [
        ("Simple import", "import foo.bar"),
        ("Function call", "eval('test')"),
        ("String literal", 'message = "hello world"'),
        ("Number literal", "x = 42"),
        ("From import", "from foo.bar import baz"),
    ]
    
    for name, code in test_cases:
        print(f"\n--- {name}: {code} ---")
        tree = parser.parse(bytes(code, "utf8"))
        print_ast(tree.root_node, code)

def analyze_java_code():
    print("\n\n=== JAVA AST ANALYSIS ===")
    
    # Initialize Java parser
    JAVA_LANGUAGE = Language(tsjava.language(), "java")
    parser = Parser()
    parser.set_language(JAVA_LANGUAGE)
    
    # Test different Java constructs
    test_cases = [
        ("Method call", "System.out.println(message);"),
        ("String literal", 'String s = "hello world";'),
        ("Number literal", "int x = 42;"),
    ]
    
    for name, code in test_cases:
        print(f"\n--- {name}: {code} ---")
        tree = parser.parse(bytes(code, "utf8"))
        print_ast(tree.root_node, code)

if __name__ == "__main__":
    analyze_python_code()
    analyze_java_code()
