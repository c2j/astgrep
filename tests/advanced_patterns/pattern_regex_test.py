#!/usr/bin/env python3
"""
Test file for pattern-regex functionality.
This file contains various string patterns to test regex matching.
"""

# Test 1: API keys (should match api-keys-regex)
def test_api_keys():
    # Should match: 32+ character alphanumeric strings
    openai_key = "sk-1234567890abcdefghijklmnopqrstuvwxyz"  # Should match
    stripe_key = "sk_test_1234567890abcdefghijklmnopqrstuvwxyz"  # Should match
    aws_secret = "abcdefghijklmnopqrstuvwxyz1234567890ABCDEF"  # Should match
    
    # Should NOT match: too short
    short_key = "sk-123"  # Should NOT match

# Test 2: JWT tokens (should match jwt-tokens-regex)
def test_jwt_tokens():
    # Should match: JWT format
    jwt_token = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIiwibmFtZSI6IkpvaG4gRG9lIiwiaWF0IjoxNTE2MjM5MDIyfQ.SflKxwRJSMeKKF2QT4fwpMeJf36POk6yJV_adQssw5c"  # Should match
    another_jwt = "eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIxMjM0NTY3ODkwIiwibmFtZSI6IkpvaG4gRG9lIn0.Gfx6VO9tcxwk6xqx9yYzSfebfeakZp5JYIgP_edcw_A"  # Should match

# Test 3: Credit card numbers (should match credit-card-numbers)
def test_credit_cards():
    # Should match: various credit card formats
    visa_card = "4532 1234 5678 9012"  # Should match
    mastercard = "5555-1234-5678-9012"  # Should match
    amex_card = "3782 822463 10005"  # Should match
    discover_card = "6011123456789012"  # Should match

# Test 4: Email addresses (should match email-addresses)
def test_emails():
    # Should match: valid email formats
    email1 = "user@example.com"  # Should match
    email2 = "john.doe+test@company.co.uk"  # Should match
    email3 = "admin123@subdomain.example.org"  # Should match
    
    # Edge cases
    complex_email = "user.name+tag@sub.domain.example.com"  # Should match

# Test 5: IP addresses (should match ip-addresses)
def test_ip_addresses():
    # Should match: IPv4 addresses
    local_ip = "192.168.1.1"  # Should match
    public_ip = "8.8.8.8"  # Should match
    server_ip = "203.0.113.42"  # Should match
    localhost = "127.0.0.1"  # Should match

# Test 6: Phone numbers (should match phone-numbers)
def test_phone_numbers():
    # Should match: various phone formats
    us_phone1 = "(555) 123-4567"  # Should match
    us_phone2 = "555-123-4567"  # Should match
    us_phone3 = "555.123.4567"  # Should match
    us_phone4 = "+1-555-123-4567"  # Should match
    us_phone5 = "5551234567"  # Should match

# Test 7: Social Security Numbers (should match social-security-numbers)
def test_ssn():
    # Should match: SSN format
    ssn1 = "123-45-6789"  # Should match
    ssn2 = "987-65-4321"  # Should match

# Test 8: AWS access keys (should match aws-access-keys)
def test_aws_keys():
    # Should match: AWS access key format
    aws_key1 = "AKIAIOSFODNN7EXAMPLE"  # Should match
    aws_key2 = "AKIAI44QH8DHBEXAMPLE"  # Should match

# Test 9: GitHub tokens (should match github-tokens)
def test_github_tokens():
    # Should match: GitHub personal access token format
    github_token = "ghp_1234567890abcdefghijklmnopqrstuvwxyz"  # Should match

# Test 10: Slack tokens (should match slack-tokens)
def test_slack_tokens():
    # Should match: various Slack token formats
    
   pass

# Test 11: Password patterns (should match password-patterns)
def test_password_patterns():
    # Should match: strings containing password/secret/token
    password_string = "Password123!"  # Should match
    secret_string = "Secret_API_Key"  # Should match
    token_string = "Token_Bearer_xyz"  # Should match
    
    # Case variations
    password_lower = "password"  # Should match
    secret_upper = "SECRET"  # Should match

# Test 12: SQL injection patterns (should match sql-injection-patterns)
def test_sql_injection():
    # Should match: SQL injection attempts
    sql_inject1 = "'; DROP TABLE users; --"  # Should match
    sql_inject2 = "' UNION SELECT * FROM passwords --"  # Should match
    sql_inject3 = "1' OR '1'='1"  # Should match

# Test 13: XSS patterns (should match xss-patterns)
def test_xss_patterns():
    # Should match: XSS attack patterns
    script_tag = "<script>alert('XSS')</script>"  # Should match
    javascript_url = "javascript:alert('XSS')"  # Should match
    onclick_event = "onclick=alert('XSS')"  # Should match

# Test 14: Sensitive file paths (should match file-paths-sensitive)
def test_sensitive_paths():
    # Should match: sensitive system files
    passwd_file = "/etc/passwd"  # Should match
    shadow_file = "/etc/shadow"  # Should match
    windows_system = "C:\\Windows\\System32\\config\\SAM"  # Should match

# Test 15: URLs (should match url-patterns)
def test_urls():
    # Should match: HTTP/HTTPS URLs
    http_url = "http://example.com/path?param=value"  # Should match
    https_url = "https://api.example.com/v1/users"  # Should match
    complex_url = "https://subdomain.example.com:8080/path/to/resource?q=search&limit=10"  # Should match

# Test 16: Base64 patterns (should match base64-patterns)
def test_base64():
    # Should match: Base64 encoded data
    base64_data = "SGVsbG8gV29ybGQhIFRoaXMgaXMgYSBsb25nIGJhc2U2NCBlbmNvZGVkIHN0cmluZw=="  # Should match
    base64_key = "YWJjZGVmZ2hpamtsbW5vcHFyc3R1dnd4eXoxMjM0NTY3ODkw"  # Should match

# Test 17: Hexadecimal patterns (should match hex-patterns)
def test_hex_patterns():
    # Should match: Long hex strings
    hex_hash = "a1b2c3d4e5f6789012345678901234567890abcdef"  # Should match
    md5_hash = "5d41402abc4b2a76b9719d911017c592"  # Should match
    sha256_hash = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"  # Should match

# Test 18: Version numbers (should match version-numbers)
def test_versions():
    # Should match: semantic version numbers
    version1 = "1.0.0"  # Should match
    version2 = "2.15.3"  # Should match
    version3 = "10.2.1"  # Should match

# Test 19: UUIDs (should match uuid-patterns)
def test_uuids():
    # Should match: UUID format
    uuid1 = "550e8400-e29b-41d4-a716-446655440000"  # Should match
    uuid2 = "6ba7b810-9dad-11d1-80b4-00c04fd430c8"  # Should match
    uuid3 = "f47ac10b-58cc-4372-a567-0e02b2c3d479"  # Should match

# Test 20: Case insensitive patterns (should match case-insensitive-regex)
def test_case_insensitive():
    # Should match: various case combinations of "password"
    password_mixed = "PaSSwoRD"  # Should match
    password_upper = "PASSWORD"  # Should match
    password_lower = "password"  # Should match

# Test 21: Strings that should NOT match
def test_non_matches():
    # These should not match most patterns
    short_string = "abc"  # Too short for most patterns
    normal_text = "This is just normal text"  # No special patterns
    number_only = "123"  # Just a number
    empty_string = ""  # Empty string

# Test 22: Edge cases and boundary conditions
def test_edge_cases():
    # Boundary cases for different patterns
    exactly_32_chars = "abcdefghijklmnopqrstuvwxyz123456"  # Exactly 32 chars
    just_under_32 = "abcdefghijklmnopqrstuvwxyz12345"  # 31 chars
    just_over_32 = "abcdefghijklmnopqrstuvwxyz1234567"  # 33 chars
    
    # Malformed patterns
    malformed_email = "user@"  # Incomplete email
    malformed_ip = "999.999.999.999"  # Invalid IP
    malformed_phone = "123-45"  # Incomplete phone

if __name__ == "__main__":
    print("Running pattern-regex tests...")
    
    # Expected matches per pattern:
    # - api-keys-regex: 3 matches
    # - jwt-tokens-regex: 2 matches
    # - credit-card-numbers: 4 matches
    # - email-addresses: 4 matches
    # - ip-addresses: 4 matches
    # - phone-numbers: 5 matches
    # - social-security-numbers: 2 matches
    # - aws-access-keys: 2 matches
    # - github-tokens: 1 match
    # - slack-tokens: 3 matches
    # - password-patterns: 5 matches
    # - sql-injection-patterns: 3 matches
    # - xss-patterns: 3 matches
    # - file-paths-sensitive: 3 matches
    # - url-patterns: 3 matches
    # - base64-patterns: 2 matches
    # - hex-patterns: 3 matches
    # - version-numbers: 3 matches
    # - uuid-patterns: 3 matches
    # - case-insensitive-regex: 3 matches
    
    test_api_keys()
    test_jwt_tokens()
    test_credit_cards()
    test_emails()
    test_ip_addresses()
    test_phone_numbers()
    test_ssn()
    test_aws_keys()
    test_github_tokens()
    test_slack_tokens()
    test_password_patterns()
    test_sql_injection()
    test_xss_patterns()
    test_sensitive_paths()
    test_urls()
    test_base64()
    test_hex_patterns()
    test_versions()
    test_uuids()
    test_case_insensitive()
    test_non_matches()
    test_edge_cases()
    
    print("Pattern-regex tests completed.")
