# Test file for focus-metavariable functionality

import requests
import sqlite3

# Test focusing on sensitive parameters
def process_data(username, password, email):
    # Should focus only on 'password' parameter
    return authenticate(username, password, email)

def login_user(user_id, secret_key, session_data):
    # Should focus only on 'secret_key' parameter
    return validate_credentials(user_id, secret_key, session_data)

def update_profile(name, api_token, preferences):
    # Should focus only on 'api_token' parameter
    return save_profile(name, api_token, preferences)

def safe_function(data, config, options):
    # Should NOT match - no sensitive parameters
    return process_safely(data, config, options)

# Test focusing on dangerous arguments
def test_dangerous_calls():
    # Should focus on the eval argument
    dangerous_function("safe", "eval(user_input)", "other")
    
    # Should focus on the exec argument
    dangerous_function("param1", "exec(malicious_code)", "param3")
    
    # Should focus on the system argument
    dangerous_function("data", "system('rm -rf /')", "config")
    
    # Should NOT match - no dangerous arguments
    dangerous_function("safe1", "safe_operation", "safe3")

# Test SQL query focus
def database_operations():
    cursor = sqlite3.cursor()
    
    # Should focus on the query with string concatenation
    query = "SELECT * FROM users WHERE id = " + user_id
    cursor.execute(query, params)
    
    # Should focus on this concatenated query too
    cursor.execute("DELETE FROM " + table_name + " WHERE id = ?", (user_id,))
    
    # Should NOT match - parameterized query
    cursor.execute("SELECT * FROM users WHERE id = ?", (user_id,))

# Test URL endpoint focus
def http_requests():
    # Should focus on insecure HTTP URLs
    response = requests.get("http://api.example.com/data", headers=headers)
    
    # Should focus on this HTTP URL too
    requests.get("http://insecure.service.com/endpoint", auth=auth)
    
    # Should NOT match - secure HTTPS URL
    requests.get("https://secure.api.com/data", headers=headers)
