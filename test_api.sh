#!/bin/bash

echo "Testing CR Web Service API"
echo "=========================="

# Test health endpoint
echo "1. Testing health endpoint..."
curl -s http://127.0.0.1:8080/health | jq .
echo ""

# Test analyze endpoint with JavaScript code
echo "2. Testing analyze endpoint with JavaScript code..."
curl -s -X POST http://127.0.0.1:8080/api/v1/analyze \
  -H "Content-Type: application/json" \
  -d '{
    "code": "function unsafeFunction(userInput) {\n    const query = \"SELECT * FROM users WHERE id = \" + userInput;\n    document.getElementById(\"output\").innerHTML = userInput;\n    return query;\n}",
    "language": "javascript",
    "rules": []
  }' | jq .
echo ""

# Test analyze endpoint with file upload
echo "3. Testing analyze endpoint with file upload..."
curl -s -X POST http://127.0.0.1:8080/api/v1/analyze/file \
  -F "file=@test_code.js" \
  -F "language=javascript" | jq .
echo ""

# Test rules endpoint
echo "4. Testing rules endpoint..."
curl -s http://127.0.0.1:8080/api/v1/rules | jq .
echo ""

echo "API testing completed!"
