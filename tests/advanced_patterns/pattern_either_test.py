#!/usr/bin/env python3
"""
Test file for pattern-either functionality.
This file contains various code patterns that should be matched by the either patterns.
"""

import hashlib
import requests
import urllib.request
import codecs
import io
from Crypto.Hash import MD5, SHA1

# Test 1: Dangerous function calls (should match 4 patterns)
def test_dangerous_functions():
    # Should match: eval($CODE)
    result1 = eval("1 + 1")
    
    # Should match: exec($CMD)
    exec("print('hello')")
    
    # Should match: compile($CODE, $FILE, $MODE)
    code_obj = compile("x = 1", "test.py", "exec")
    
    # Should match: __import__($MODULE)
    os_module = __import__("os")
    
    return result1

# Test 2: Weak crypto algorithms (should match 4 patterns)
def test_weak_crypto():
    data = b"test data"
    
    # Should match: hashlib.md5($DATA)
    md5_hash = hashlib.md5(data)
    
    # Should match: hashlib.sha1($DATA)
    sha1_hash = hashlib.sha1(data)
    
    # Should match: Crypto.Hash.MD5.new($DATA)
    crypto_md5 = MD5.new(data)
    
    # Should match: Crypto.Hash.SHA1.new($DATA)
    crypto_sha1 = SHA1.new(data)
    
    return md5_hash, sha1_hash, crypto_md5, crypto_sha1

# Test 3: SQL injection patterns (should match 3 patterns)
def test_sql_injection():
    user_input = "'; DROP TABLE users; --"
    
    # Should match: cursor.execute($QUERY + $VAR)
    cursor.execute("SELECT * FROM users WHERE name = " + user_input)
    
    # Should match: cursor.execute($QUERY % $VAR)
    cursor.execute("SELECT * FROM users WHERE id = %s" % user_input)
    
    # Should match: cursor.execute(f"$QUERY{$VAR}")
    cursor.execute(f"SELECT * FROM users WHERE email = {user_input}")

# Test 4: File operations (should match 4 patterns)
def test_file_operations():
    filename = "test.txt"
    mode = "r"
    
    # Should match: open($FILE, $MODE)
    f1 = open(filename, mode)
    
    # Should match: file($FILE, $MODE) - Python 2 style
    # f2 = file(filename, mode)  # Commented out for Python 3 compatibility
    
    # Should match: codecs.open($FILE, $MODE)
    f3 = codecs.open(filename, mode, encoding='utf-8')
    
    # Should match: io.open($FILE, $MODE)
    f4 = io.open(filename, mode)
    
    return f1, f3, f4

# Test 5: Network requests (should match 3 patterns)
def test_network_requests():
    url = "https://api.example.com/data"
    data = {"key": "value"}
    
    # Should match: requests.get($URL)
    response1 = requests.get(url)
    
    # Should match: requests.post($URL, $DATA)
    response2 = requests.post(url, data)
    
    # Should match: urllib.request.urlopen($URL)
    response3 = urllib.request.urlopen(url)
    
    return response1, response2, response3

# Test 6: Complex nested either patterns
def test_complex_nested():
    obj = SomeObject()
    
    # Should match complex pattern
    obj.dangerous_method(eval("malicious_code"))
    
    # Should match another complex pattern
    compile("code", "file", "exec")

# Test 7: String patterns (should match 4 patterns but exclude assignments)
def test_string_patterns():
    # These should match (not assignments)
    check_password("PASSWORD")
    validate_secret("SECRET")
    authenticate("API_KEY")
    verify_token("TOKEN")
    
    # These should NOT match (assignments with pattern-not)
    password = "my_password"
    secret = "my_secret"
    api_key = "my_api_key"
    token = "my_token"

# Test 8: Safe functions (should NOT match any either patterns)
def test_safe_functions():
    # These should not match any dangerous patterns
    print("Hello, world!")
    len([1, 2, 3])
    str(42)
    int("123")
    
    # Safe crypto
    hashlib.sha256(b"data")
    hashlib.sha512(b"data")
    
    # Safe file operations (different patterns)
    with open("file.txt") as f:
        content = f.read()

# Test 9: Edge cases
def test_edge_cases():
    # Partial matches that should not trigger
    evaluation = "eval_function"  # Contains "eval" but not a call
    execution = "exec_command"    # Contains "exec" but not a call
    
    # Method calls that look similar but are different
    obj.eval_method()  # Not the built-in eval
    obj.exec_method()  # Not the built-in exec

if __name__ == "__main__":
    print("Running pattern-either tests...")
    
    # Expected matches:
    # - dangerous-function-calls: 4 matches
    # - crypto-weak-algorithms: 4 matches  
    # - sql-injection-patterns: 3 matches
    # - file-operations-either: 3 matches
    # - network-requests-either: 3 matches
    # - complex-either-nested: 2 matches
    # - string-patterns-either: 4 matches
    
    test_dangerous_functions()
    test_weak_crypto()
    test_sql_injection()
    test_file_operations()
    test_network_requests()
    test_complex_nested()
    test_string_patterns()
    test_safe_functions()
    test_edge_cases()
    
    print("Pattern-either tests completed.")
