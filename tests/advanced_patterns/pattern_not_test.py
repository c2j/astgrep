#!/usr/bin/env python3
"""
Test file for pattern-not functionality.
This file contains various code patterns to test exclusion logic.
"""

import requests
import hashlib
import sqlite3
import os
import sys
import json
import re
import datetime
import custom_module

# Test 1: Function calls except safe ones
def test_function_calls_except_safe():
    # These should NOT match (safe functions)
    print("Hello, world!")
    length = len([1, 2, 3])
    text = str(42)
    number = int("123")
    decimal = float("3.14")
    
    # These SHOULD match (not in exclusion list)
    result = eval("1 + 1")  # Should match
    exec("print('executed')")  # Should match
    hash_obj = hashlib.md5(b"data")  # Should match
    response = requests.get("http://example.com")  # Should match

# Test 2: Imports except standard library
def test_imports_except_standard():
    # These should NOT match (standard library - excluded)
    # import os        # Already imported above
    # import sys       # Already imported above  
    # import json      # Already imported above
    # import re        # Already imported above
    # import datetime  # Already imported above
    
    # These SHOULD match (not standard library)
    # import custom_module    # Already imported above - should match
    # import requests         # Already imported above - should match
    # import hashlib          # Already imported above - should match
    pass

# Test 3: String literals except safe ones
def test_string_literals_except_safe():
    # These should NOT match (safe strings - excluded)
    greeting = "hello"
    place = "world"
    mode = "test"
    empty = ""
    
    # These SHOULD match (not in exclusion list)
    password = "secret123"  # Should match
    api_key = "sk-1234567890"  # Should match
    token = "bearer_token_xyz"  # Should match
    query = "SELECT * FROM users WHERE id = 1"  # Should match

# Test 4: Assignments except constants
def test_assignments_except_constants():
    # These should NOT match (constants - excluded)
    flag1 = True
    flag2 = False
    value = None
    zero = 0
    one = 1
    
    # These SHOULD match (not constants)
    name = "John"  # Should match
    age = 25  # Should match
    items = [1, 2, 3]  # Should match
    config = {"key": "value"}  # Should match

# Test 5: Method calls except getters
def test_method_calls_except_getters():
    obj = SomeObject()
    
    # These should NOT match (getter methods - excluded)
    value = obj.get("key")
    name = obj.getValue()
    type_info = obj.getName()
    obj_type = obj.getType()
    
    # These SHOULD match (not getter methods)
    obj.set("key", "value")  # Should match
    obj.delete("item")  # Should match
    obj.update({"key": "new_value"})  # Should match
    obj.process()  # Should match

# Test 6: File operations except read-only
def test_file_operations_except_read_only():
    filename = "test.txt"
    
    # These should NOT match (read-only modes - excluded)
    f1 = open(filename, "r")
    f2 = open(filename, "rb")
    
    # These SHOULD match (write/append modes)
    f3 = open(filename, "w")  # Should match
    f4 = open(filename, "a")  # Should match
    f5 = open(filename, "r+")  # Should match
    f6 = open(filename, "wb")  # Should match

# Test 7: Network requests except GET
def test_network_requests_except_get():
    url = "https://api.example.com"
    data = {"key": "value"}
    
    # These should NOT match (GET requests - excluded)
    response1 = requests.get(url)
    response2 = requests.get(url, params={"q": "search"})
    
    # These SHOULD match (non-GET requests)
    response3 = requests.post(url, data=data)  # Should match
    response4 = requests.put(url, json=data)  # Should match
    response5 = requests.delete(url)  # Should match
    response6 = requests.patch(url, data=data)  # Should match

# Test 8: Complex NOT with EITHER - dangerous functions not in tests
def test_complex_not_with_either():
    # These SHOULD match (dangerous functions outside test context)
    eval("dangerous_code")  # Should match
    exec("malicious_command")  # Should match
    compile("code", "file", "exec")  # Should match

# Test 9: Test context (should be excluded by complex patterns)
def test_dangerous_in_test_context():
    # These should NOT match (inside test function)
    eval("test_code")  # Should NOT match (in test function)
    exec("test_command")  # Should NOT match (in test function)

class TestClass:
    def test_method(self):
        # These should NOT match (inside test class)
        eval("class_test_code")  # Should NOT match (in test class)
        exec("class_test_command")  # Should NOT match (in test class)

# Test 10: SQL queries except safe ones
def test_sql_queries_except_safe():
    cursor = sqlite3.connect(":memory:").cursor()
    
    # These should NOT match (safe queries - excluded)
    cursor.execute("SELECT * FROM users")
    cursor.execute("INSERT INTO logs ...")
    cursor.execute("UPDATE settings ...")
    cursor.execute("SELECT name FROM users WHERE id = ?", (1,))  # Parameterized
    
    # These SHOULD match (potentially unsafe queries)
    user_input = "'; DROP TABLE users; --"
    cursor.execute(f"SELECT * FROM users WHERE name = '{user_input}'")  # Should match
    cursor.execute("DELETE FROM users")  # Should match
    cursor.execute("DROP TABLE sensitive_data")  # Should match

# Test 11: Edge cases and boundary conditions
def test_edge_cases():
    # Functions with similar names but should still match
    my_print = lambda x: x  # Different from print()
    my_len = lambda x: 42   # Different from len()
    
    result1 = my_print("test")  # Should match (not built-in print)
    result2 = my_len([1, 2, 3])  # Should match (not built-in len)
    
    # Nested function calls
    result3 = str(len([1, 2, 3]))  # str() should match, len() should not
    
    # Method calls on built-in functions
    print.write = lambda x: x  # Hypothetical
    # print.write("test")  # Would match if this were valid

if __name__ == "__main__":
    print("Running pattern-not tests...")
    
    # Expected behavior:
    # - Functions except safe ones: should match eval, exec, hashlib.md5, requests.get
    # - Imports except standard: should match custom_module, requests, hashlib
    # - Strings except safe: should match password, api_key, token, query
    # - Assignments except constants: should match name, age, items, config
    # - Methods except getters: should match set, delete, update, process
    # - File ops except read-only: should match "w", "a", "r+", "wb" modes
    # - Network except GET: should match post, put, delete, patch
    # - Complex patterns: should exclude test contexts
    # - SQL except safe: should match unsafe queries
    
    test_function_calls_except_safe()
    test_imports_except_standard()
    test_string_literals_except_safe()
    test_assignments_except_constants()
    test_method_calls_except_getters()
    test_file_operations_except_read_only()
    test_network_requests_except_get()
    test_complex_not_with_either()
    test_dangerous_in_test_context()
    test_sql_queries_except_safe()
    test_edge_cases()
    
    print("Pattern-not tests completed.")
