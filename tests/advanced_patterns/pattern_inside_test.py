#!/usr/bin/env python3
"""
Test file for pattern-inside functionality.
This file contains various nested code patterns to test context matching.
"""

import requests
import urllib.request
import pickle
import yaml
import subprocess
import sqlite3
from flask import Flask

app = Flask(__name__)

# Test 1: eval() inside function definition (should match)
def dangerous_function():
    user_input = input("Enter code: ")
    result = eval(user_input)  # Should match: eval-inside-function
    return result

def another_function():
    code = "1 + 1"
    return eval(code)  # Should match: eval-inside-function

# Test 2: SQL injection inside web route (should match)
@app.route('/users/<user_id>')
def get_user(user_id):
    cursor = get_db_cursor()
    # Should match: sql-injection-inside-route
    query = "SELECT * FROM users WHERE id = " + user_id
    cursor.execute(query + user_id)
    return cursor.fetchone()

@app.route('/search')
def search_users():
    search_term = request.args.get('q')
    cursor = get_db_cursor()
    # Should match: sql-injection-inside-route
    cursor.execute("SELECT * FROM users WHERE name = " + search_term)
    return cursor.fetchall()

# Test 3: Dangerous calls inside try block (should match)
def risky_operation():
    try:
        eval("dangerous_code")  # Should match: dangerous-calls-inside-try
        exec("malicious_command")  # Should match: dangerous-calls-inside-try
        __import__("suspicious_module")  # Should match: dangerous-calls-inside-try
    except Exception as e:
        print(f"Error: {e}")

def safe_operation():
    try:
        result = int("123")  # Should NOT match (not dangerous)
        return result
    except ValueError:
        return 0

# Test 4: File write inside loop (should match)
def write_logs():
    log_file = open("app.log", "w")
    events = ["event1", "event2", "event3"]
    
    for event in events:
        log_file.write(f"Log: {event}\n")  # Should match: file-write-inside-loop
    
    log_file.close()

def batch_write():
    data_file = open("data.txt", "w")
    items = range(100)
    
    for item in items:
        data_file.write(str(item))  # Should match: file-write-inside-loop

# Test 5: Network requests inside loop (should match)
def fetch_multiple_urls():
    urls = ["http://api1.com", "http://api2.com", "http://api3.com"]
    
    for url in urls:
        response = requests.get(url)  # Should match: network-request-inside-loop
        print(response.status_code)

def post_multiple_data():
    endpoints = ["http://api.com/users", "http://api.com/posts"]
    data = {"key": "value"}
    
    for endpoint in endpoints:
        requests.post(endpoint, data)  # Should match: network-request-inside-loop

# Test 6: Hardcoded passwords inside class (should match)
class DatabaseConfig:
    def __init__(self):
        self.host = "localhost"
        self.port = 5432
        self.password = "admin123"  # Should match: password-hardcoded-inside-class
        self.secret = "secret_key"  # Should match: password-hardcoded-inside-class

class APIClient:
    api_key = "sk-1234567890abcdef"  # Should match: password-hardcoded-inside-class
    
    def __init__(self):
        self.token = "bearer_token"

# Test 7: Unsafe deserialization inside handler (should match)
def handle_upload(file_data):
    # Should match: unsafe-deserialization-inside-handler
    data = pickle.loads(file_data)
    return data

def handle_config(config_file):
    with open(config_file, 'r') as f:
        # Should match: unsafe-deserialization-inside-handler
        config = yaml.load(f)
    return config

def handle_request(request_data):
    # Should match: unsafe-deserialization-inside-handler
    obj = pickle.load(request_data)
    return obj

# Test 8: Shell command inside subprocess (should match)
def execute_command(user_command):
    # Should match: shell-command-inside-subprocess
    result = subprocess.run(user_command, shell=True)
    return result

def run_script(script_name):
    # Should match: shell-command-inside-subprocess
    subprocess.call(f"python {script_name}", shell=True)

# Test 9: eval() inside conditional (should match)
def conditional_eval():
    user_input = input("Enter expression: ")
    
    if user_input:
        result = eval(user_input)  # Should match: eval-inside-conditional
        return result
    
    return None

def debug_mode():
    debug = True
    
    if debug:
        eval("debug_expression")  # Should match: eval-inside-conditional

# Test 10: Database query inside transaction (should match)
def transfer_money(from_account, to_account, amount):
    db = get_database()
    
    with db.transaction():
        cursor = db.cursor()
        # Should match: database-query-inside-transaction
        cursor.execute("UPDATE accounts SET balance = balance - ? WHERE id = ?", 
                      (amount, from_account))
        # Should match: database-query-inside-transaction
        cursor.execute("UPDATE accounts SET balance = balance + ? WHERE id = ?", 
                      (amount, to_account))

# Test 11: Nested inside patterns (should match)
class SecurityManager:
    def validate_input(self, user_input):
        # Should match: nested-inside-patterns (eval inside function inside class)
        if self.is_safe(user_input):
            return eval(user_input)
        return None

# Test 12: Exception handling inside function (should match)
def error_handler():
    try:
        risky_operation()
    except ValueError as e:
        # Should match: exception-handling-inside-function
        raise RuntimeError("Processing failed")
    except Exception as e:
        # Should match: exception-handling-inside-function
        raise SystemError("System error occurred")

# Test 13: Async/await inside function (should match)
async def fetch_data():
    # Should match: async-await-inside-function
    response = await requests.get("http://api.com/data")
    return response

async def process_async():
    # Should match: async-await-inside-function
    result = await some_async_function()
    return result

# Test 14: Context manager inside function (should match)
def read_file():
    # Should match: context-manager-inside-function
    with open("data.txt", "r") as file:
        content = file.read()
    return content

def database_operation():
    # Should match: context-manager-inside-function
    with get_db_connection() as conn:
        cursor = conn.cursor()
        cursor.execute("SELECT * FROM users")

# Test 15: Patterns that should NOT match (outside context)
# These are outside functions/classes/loops, so should not match inside patterns

eval("global_eval")  # Should NOT match inside patterns
exec("global_exec")  # Should NOT match inside patterns

password = "global_password"  # Should NOT match (not inside class)

cursor = sqlite3.connect(":memory:").cursor()
cursor.execute("SELECT 1")  # Should NOT match (not inside transaction)

# Global network request (not in loop)
response = requests.get("http://example.com")  # Should NOT match

if __name__ == "__main__":
    print("Running pattern-inside tests...")
    
    # Expected matches:
    # - eval-inside-function: 4 matches
    # - sql-injection-inside-route: 2 matches
    # - dangerous-calls-inside-try: 3 matches
    # - file-write-inside-loop: 2 matches
    # - network-request-inside-loop: 2 matches
    # - password-hardcoded-inside-class: 3 matches
    # - unsafe-deserialization-inside-handler: 3 matches
    # - shell-command-inside-subprocess: 2 matches
    # - eval-inside-conditional: 2 matches
    # - database-query-inside-transaction: 2 matches
    # - nested-inside-patterns: 1 match
    # - exception-handling-inside-function: 2 matches
    # - async-await-inside-function: 2 matches
    # - context-manager-inside-function: 2 matches
    
    print("Pattern-inside tests completed.")
