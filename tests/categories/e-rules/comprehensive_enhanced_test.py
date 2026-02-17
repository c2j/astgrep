# Comprehensive test file for enhanced pattern features

import requests
import sqlite3
import os

# Test complex security check
def vulnerable_function():
    # Should match - execute with string concatenation outside try-catch
    db.execute("SELECT * FROM users WHERE id = " + user_id)
    
    # Should match - eval with concatenation outside try-catch
    result = eval("process_" + operation + "(data)")

def test_function():
    # Should NOT match - excluded by pattern-not-regex (test_.*)
    db.execute("SELECT * FROM test_table WHERE id = " + test_id)

def safe_function():
    try:
        # Should NOT match - inside try-catch block
        db.execute("DELETE FROM users WHERE name = " + username)
    except Exception as e:
        handle_error(e)

# Test unsafe HTTP outside dev
def production_code():
    # Should match - HTTP request outside development context
    response = requests.get("http://api.production.com/data", headers=headers)

def development_code():
    if os.environ.get("ENV") == "development":
        # Should NOT match - inside development context
        response = requests.get("http://api.staging.com/data", headers=headers)

def test_api_call():
    # Should NOT match - test function excluded
    response = requests.get("http://api.test.com/data", headers=headers)

def local_testing():
    # Should NOT match - localhost excluded by pattern-not-regex
    response = requests.get("http://localhost:8080/api", headers=headers)
    
    # Should NOT match - 127.0.0.1 excluded by pattern-not-regex
    response = requests.get("http://127.0.0.1:3000/data", headers=headers)

# Test advanced SQL injection
def database_operations():
    cursor = sqlite3.cursor()
    
    # Should match - string concatenation outside transaction
    query = "SELECT * FROM users WHERE name = '" + username + "'"
    cursor.execute(query, params)
    
    # Should match - another concatenation pattern
    cursor.execute("DELETE FROM " + table + " WHERE id = '" + user_id + "'", params)

def safe_database_operations():
    connection = sqlite3.connect("db.sqlite")
    with connection.begin():
        cursor = connection.cursor()
        # Should NOT match - inside transaction block
        query = "SELECT * FROM users WHERE name = '" + username + "'"
        cursor.execute(query, params)

def test_database():
    # Should NOT match - test function excluded
    cursor = sqlite3.cursor()
    query = "SELECT * FROM test_users WHERE name = '" + test_name + "'"
    cursor.execute(query, params)
