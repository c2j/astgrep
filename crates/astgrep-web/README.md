# CR Web Service

A RESTful web service for code analysis using the CR (Code Review) semantic analysis engine.

## Features

- **Code Analysis**: Analyze code snippets or files for security vulnerabilities and code quality issues
- **Multiple Languages**: Support for JavaScript, Python, Java, and more
- **Rule Management**: Manage and configure analysis rules
- **RESTful API**: Clean REST API with comprehensive documentation
- **File Upload**: Support for analyzing uploaded files
- **Real-time Analysis**: Fast analysis with detailed results

## Quick Start

### Prerequisites

- Rust 1.70+ 
- Cargo

### Building and Running

1. **Build the service:**
   ```bash
   cargo build -p cr-web --bin cr-web
   ```

2. **Create rules directory:**
   ```bash
   mkdir -p rules
   ```

3. **Start the server:**
   ```bash
   cargo run -p cr-web --bin cr-web
   ```

The server will start on `http://127.0.0.1:8080` by default.

### API Documentation

Once the server is running, you can access:

- **API Documentation**: http://127.0.0.1:8080/docs
- **Health Check**: http://127.0.0.1:8080/health

## API Endpoints

### Health Check
```bash
GET /health
```

### Code Analysis
```bash
POST /api/v1/analyze
Content-Type: application/json

{
  "code": "function example() { return eval(userInput); }",
  "language": "javascript",
  "rules": []
}
```

### File Analysis
```bash
POST /api/v1/analyze/file
Content-Type: multipart/form-data

file: <uploaded_file>
language: javascript
```

### List Rules
```bash
GET /api/v1/rules
```

### Get Analysis Job
```bash
GET /api/v1/jobs/{job_id}
```

## Example Usage

### Analyze JavaScript Code
```bash
curl -X POST http://127.0.0.1:8080/api/v1/analyze \
  -H "Content-Type: application/json" \
  -d '{
    "code": "function unsafe(input) { return eval(input); }",
    "language": "javascript"
  }'
```

### Upload and Analyze File
```bash
curl -X POST http://127.0.0.1:8080/api/v1/analyze/file \
  -F "file=@example.js" \
  -F "language=javascript"
```

## Configuration

The service can be configured through environment variables or configuration files. Key settings include:

- `BIND_ADDRESS`: Server bind address (default: 127.0.0.1:8080)
- `RULES_DIRECTORY`: Directory containing analysis rules (default: rules)
- `MAX_UPLOAD_SIZE`: Maximum file upload size (default: 100MB)
- `REQUEST_TIMEOUT`: Request timeout in seconds (default: 300)

## Development

### Running Tests
```bash
cargo test -p cr-web
```

### Building for Production
```bash
cargo build --release -p cr-web --bin cr-web
```

## License

This project is licensed under the MIT License.
