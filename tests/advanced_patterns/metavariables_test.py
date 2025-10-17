#!/usr/bin/env python3
"""
Test file for metavariables functionality.
This file contains various code patterns to test metavariable binding and constraints.
"""

import subprocess
import os
import hashlib
import crypto
import requests

# Test 1: Dangerous function with regex (should match dangerous-function-with-regex)
def test_dangerous_functions():
    # Should match: $FUNC matches "eval|exec|compile"
    result1 = eval("1 + 1")  # Should match
    exec("print('hello')")  # Should match
    code_obj = compile("x = 1", "test.py", "exec")  # Should match
    
    # Should NOT match: function name doesn't match regex
    print("safe function")  # Should NOT match
    len([1, 2, 3])  # Should NOT match

# Test 2: Assignment with dangerous value (should match assignment-with-dangerous-value)
def test_dangerous_assignments():
    # Should match: $FUNC contains "dangerous|unsafe|risky"
    result1 = dangerous_function()  # Should match
    result2 = unsafe_operation()  # Should match
    result3 = risky_calculation()  # Should match
    
    # Should NOT match: function name doesn't contain dangerous words
    result4 = safe_function()  # Should NOT match
    result5 = normal_operation()  # Should NOT match

# Test 3: Method call with sensitive object (should match method-call-with-sensitive-object)
def test_sensitive_objects():
    # Should match: $OBJ contains "password|secret|key|token"
    password_manager.get_password()  # Should match
    secret_store.retrieve()  # Should match
    api_key_vault.fetch()  # Should match
    token_service.validate()  # Should match
    
    # Should NOT match: object name doesn't contain sensitive words
    user_service.get_info()  # Should NOT match
    data_manager.process()  # Should NOT match

# Test 4: String with credentials (should match string-with-credentials)
def test_credential_strings():
    # Should match: $VALUE contains "password|secret|key|token"
    config1 = "my_password_123"  # Should match
    config2 = "secret_api_key"  # Should match
    config3 = "bearer_token_xyz"  # Should match
    config4 = "encryption_key_data"  # Should match
    
    # Should NOT match: value doesn't contain credential words
    config5 = "normal_string"  # Should NOT match
    config6 = "user_data"  # Should NOT match

# Test 5: Function name comparison (should match function-name-comparison)
def test_function_names():
    # Should match: function name starts with "test_"
    def test_example():  # Should match
        pass
    
    def test_another_function():  # Should match
        pass
    
    # Should NOT match: function name doesn't start with "test_"
    def example_function():  # Should NOT match
        pass
    
    def normal_function():  # Should NOT match
        pass

# Test 6: Numeric comparison (should match numeric-comparison)
def test_numeric_values():
    # Should match: $NUM > 1000
    large_number = 5000  # Should match
    huge_value = 999999  # Should match
    big_count = 1001  # Should match
    
    # Should NOT match: $NUM <= 1000
    small_number = 100  # Should NOT match
    medium_value = 500  # Should NOT match
    boundary_value = 1000  # Should NOT match

# Test 7: String length check (should match string-length-check)
def test_string_lengths():
    # Should match: string length > 10
    long_string = "this is a very long string"  # Should match
    description = "detailed description text"  # Should match
    
    # Should NOT match: string length <= 10
    short_str = "short"  # Should NOT match
    name = "John"  # Should NOT match
    exactly_ten = "1234567890"  # Should NOT match

# Test 8: Complex metavariable pattern (should match metavar-pattern-complex)
def test_complex_metavar():
    # Should match: $ARG is string containing "admin|root|system"
    user_service.authenticate("admin_user")  # Should match
    system_manager.login("root_password")  # Should match
    access_control.check("system_admin")  # Should match
    
    # Should NOT match: string doesn't contain admin/root/system
    user_service.authenticate("normal_user")  # Should NOT match
    system_manager.login("guest_access")  # Should NOT match

# Test 9: Multiple metavar constraints (should match multiple-metavar-constraints)
def test_multiple_constraints():
    # Should match: subprocess/os function with shell metacharacters
    subprocess.system("ls; rm -rf /")  # Should match
    os.popen("cat file | grep pattern")  # Should match
    subprocess.exec("command && other_command")  # Should match
    
    # Should NOT match: safe commands or wrong function
    subprocess.system("ls")  # Should NOT match (no shell metacharacters)
    print("safe function")  # Should NOT match (wrong function)

# Test 10: Variable name pattern (should match variable-name-pattern)
def test_variable_names():
    # Should match: snake_case pattern
    user_name = "John"  # Should match
    api_key = "secret"  # Should match
    file_path = "/tmp/file"  # Should match
    
    # Should NOT match: doesn't follow snake_case
    userName = "John"  # Should NOT match (camelCase)
    CONSTANT = "value"  # Should NOT match (all caps)
    x = 1  # Should NOT match (single letter)

# Test 11: Import module pattern (should match import-module-pattern)
def test_import_patterns():
    # Should match: module contains "crypto|hash|cipher"
    # import crypto  # Already imported - should match
    # import hashlib  # Already imported - should match
    import cipher_module  # Should match
    
    # Should NOT match: module doesn't contain crypto/hash/cipher
    # import requests  # Already imported - should NOT match
    import json  # Should NOT match

# Test 12: Class name convention (should match class-name-convention)
def test_class_names():
    # Should match: PascalCase
    class UserManager:  # Should match
        pass
    
    class DatabaseConnection:  # Should match
        pass
    
    class APIClient:  # Should match
        pass
    
    # Should NOT match: doesn't follow PascalCase
    class user_manager:  # Should NOT match (snake_case)
        pass
    
    class database_CONNECTION:  # Should NOT match (mixed case)
        pass

# Test 13: File operation pattern (should match file-operation-pattern)
def test_file_operations():
    # Should match: .log/.tmp/.temp files with write/append mode
    log_file = open("app.log", "w")  # Should match
    temp_file = open("data.tmp", "a")  # Should match
    temp_file2 = open("cache.temp", "w+")  # Should match
    
    # Should NOT match: wrong extension or mode
    data_file = open("data.txt", "w")  # Should NOT match (wrong extension)
    log_read = open("app.log", "r")  # Should NOT match (read mode)

# Test 14: URL pattern metavar (should match url-pattern-metavar)
def test_url_patterns():
    # Should match: URLs starting with http/https
    api_url = "https://api.example.com"  # Should match
    web_url = "http://example.com"  # Should match
    secure_url = "https://secure.site.com/path"  # Should match
    
    # Should NOT match: not HTTP URLs
    ftp_url = "ftp://files.example.com"  # Should NOT match
    file_path = "/local/file/path"  # Should NOT match

# Test 15: Nested metavar pattern (should match nested-metavar-pattern)
def test_nested_metavar():
    # Should match: method call with string containing "password|secret"
    auth_service.validate("user_password")  # Should match
    config_manager.set("api_secret")  # Should match
    
    # Should NOT match: string doesn't contain password/secret
    auth_service.validate("username")  # Should NOT match
    config_manager.set("timeout")  # Should NOT match

# Test 16: Function call chain (should match function-call-chain)
def test_call_chains():
    # Should match: get* method followed by set* method
    user.getName().setDisplayName("John")  # Should match
    config.getValue().setDefault("default")  # Should match
    
    # Should NOT match: wrong method pattern
    user.setName().getName()  # Should NOT match (wrong order)
    user.process().execute()  # Should NOT match (wrong methods)

# Test 17: Assignment type check (should match assignment-type-check)
def test_type_checking():
    # Should match: string assignments
    name = "John Doe"  # Should match
    description = "A long description"  # Should match
    
    # Should NOT match: non-string assignments
    age = 25  # Should NOT match (int)
    is_active = True  # Should NOT match (bool)
    items = [1, 2, 3]  # Should NOT match (list)

# Test 18: Conditional metavar (should match conditional-metavar)
def test_conditional_patterns():
    # Should match: comparison with True/False/None
    if flag == True:  # Should match
        pass
    
    if result == False:  # Should match
        pass
    
    if value == None:  # Should match
        pass
    
    # Should NOT match: comparison with other values
    if count == 0:  # Should NOT match
        pass
    
    if name == "John":  # Should NOT match
        pass

# Test 19: Loop variable pattern (should match loop-variable-pattern)
def test_loop_variables():
    # Should match: lowercase/underscore variable names
    for item in items:  # Should match
        pass
    
    for user_id in user_ids:  # Should match
        pass
    
    for file_name in file_names:  # Should match
        pass
    
    # Should NOT match: non-conforming variable names
    for Item in items:  # Should NOT match (capitalized)
        pass
    
    for i in range(10):  # Should match (single lowercase letter)
        pass

# Test 20: Exception type pattern (should match exception-type-pattern)
def test_exception_patterns():
    try:
        risky_operation()
    except ValueError as e:  # Should match (ends with "Error")
        pass
    except RuntimeError as err:  # Should match (ends with "Error")
        pass
    except TypeError as error:  # Should match (ends with "Error")
        pass
    
    # Should NOT match: exception doesn't end with "Error"
    except Exception as e:  # Should NOT match
        pass
    except KeyboardInterrupt as ki:  # Should NOT match
        pass

# Helper functions and classes for testing
def dangerous_function():
    return "dangerous result"

def unsafe_operation():
    return "unsafe result"

def risky_calculation():
    return "risky result"

def safe_function():
    return "safe result"

def normal_operation():
    return "normal result"

class PasswordManager:
    def get_password(self):
        return "password"

class SecretStore:
    def retrieve(self):
        return "secret"

class APIKeyVault:
    def fetch(self):
        return "key"

class TokenService:
    def validate(self):
        return True

class UserService:
    def get_info(self):
        return {}
    
    def authenticate(self, user):
        return True

class DataManager:
    def process(self):
        return "processed"

# Create instances for testing
password_manager = PasswordManager()
secret_store = SecretStore()
api_key_vault = APIKeyVault()
token_service = TokenService()
user_service = UserService()
data_manager = DataManager()

if __name__ == "__main__":
    print("Running metavariables tests...")
    
    # Expected matches per rule:
    # - dangerous-function-with-regex: 3 matches
    # - assignment-with-dangerous-value: 3 matches
    # - method-call-with-sensitive-object: 4 matches
    # - string-with-credentials: 4 matches
    # - function-name-comparison: 2 matches
    # - numeric-comparison: 3 matches
    # - string-length-check: 2 matches
    # - metavar-pattern-complex: 3 matches
    # - multiple-metavar-constraints: 3 matches
    # - variable-name-pattern: 3 matches
    # - import-module-pattern: 2 matches
    # - class-name-convention: 3 matches
    # - file-operation-pattern: 3 matches
    # - url-pattern-metavar: 3 matches
    # - nested-metavar-pattern: 2 matches
    # - function-call-chain: 2 matches
    # - assignment-type-check: 2 matches
    # - conditional-metavar: 3 matches
    # - loop-variable-pattern: 4 matches
    # - exception-type-pattern: 3 matches
    
    print("Metavariables tests completed.")
