# XML Language Support Tests

This directory contains comprehensive test cases and security rules for XML language support in cr-semservice.

## 📁 Directory Structure

```
tests/xml/
├── README.md                      # This file
├── xml_security.yaml              # Security vulnerability detection rules
├── security_test.xml              # Security test cases
├── best_practices.yaml            # XML best practices rules
├── best_practices_test.xml        # Best practices test cases
├── run_xml_tests.py              # Test runner script
├── sample.xml                     # Sample XML document
├── config.xml                     # Configuration file example
├── pom.xml                        # Maven POM file example
└── image.svg                      # SVG graphics example
```

## 🔒 Security Rules

### xml_security.yaml

Contains **15 security rules** covering:

1. **XXE Vulnerabilities (CWE-611)**
   - `xml-xxe-001`: External entity declarations
   - `xml-xxe-002`: Parameter entity declarations

2. **Hardcoded Credentials (CWE-798)**
   - `xml-hardcoded-credentials-001`: Hardcoded passwords
   - `xml-hardcoded-credentials-002`: Hardcoded API keys and tokens
   - `xml-sql-connection-string-001`: SQL connection strings with passwords

3. **Cryptographic Issues (CWE-327, CWE-319)**
   - `xml-weak-encryption-001`: Weak encryption algorithms (DES, RC4, MD5, SHA1)
   - `xml-insecure-protocol-001`: Insecure HTTP protocol usage
   - `xml-ssl-disabled-001`: SSL/TLS disabled
   - `xml-certificate-validation-disabled-001`: Certificate validation disabled

4. **Security Misconfiguration (CWE-489, CWE-209)**
   - `xml-debug-enabled-001`: Debug mode enabled
   - `xml-verbose-errors-001`: Verbose error messages
   - `xml-wildcard-cors-001`: Wildcard CORS configuration

5. **Other Security Issues**
   - `xml-xpath-injection-001`: XPath injection risks (CWE-643)
   - `xml-file-upload-path-001`: Unrestricted file upload paths (CWE-434)
   - `xml-session-timeout-001`: Excessive session timeout (CWE-613)

## ✨ Best Practices Rules

### best_practices.yaml

Contains **20 best practice rules** covering:

1. **XML Declaration**
   - `xml-best-practice-001`: Missing XML declaration
   - `xml-best-practice-002`: Missing encoding declaration
   - `xml-version-001`: Unsupported XML version
   - `xml-encoding-001`: Non-standard encoding

2. **Syntax and Structure**
   - `xml-empty-element-001`: Empty elements not self-closing
   - `xml-attribute-quotes-001`: Attributes without quotes
   - `xml-root-element-001`: Multiple root elements
   - `xml-comment-best-practice-001`: Comments with double hyphens

3. **Naming and Formatting**
   - `xml-naming-convention-001`: Element names with spaces
   - `xml-indentation-001`: Inconsistent indentation
   - `xml-attribute-order-001`: Attributes not alphabetically ordered
   - `xml-long-lines-001`: Long lines exceeding 120 characters

4. **Content Handling**
   - `xml-cdata-usage-001`: Special characters without CDATA
   - `xml-entity-reference-001`: Unescaped special characters
   - `xml-mixed-content-001`: Mixed content pattern
   - `xml-boolean-values-001`: Non-standard boolean values

5. **Namespace and Schema**
   - `xml-namespace-prefix-001`: Namespace prefix not declared
   - `xml-default-namespace-001`: Default namespace usage
   - `xml-schema-location-001`: Schema location using HTTP
   - `xml-processing-instruction-001`: Malformed processing instructions

## 🧪 Test Cases

### security_test.xml

Contains **60+ test cases** demonstrating:
- ✅ Vulnerable patterns that should be detected
- ✅ Safe alternatives and best practices
- ✅ Real-world configuration examples
- ✅ Multiple security issues in complex configurations

### best_practices_test.xml

Contains **30+ test cases** demonstrating:
- ✅ Proper XML structure and formatting
- ✅ Correct use of namespaces and schemas
- ✅ Well-formed comments and CDATA sections
- ✅ Complete configuration examples

## 🚀 Running Tests

### Quick Validation

Run the test validation script:

```bash
cd tests/xml
python3 run_xml_tests.py
```

This will:
1. Check that all required files exist
2. Validate XML syntax (if xmllint is available)
3. Validate YAML rule syntax (if PyYAML is installed)
4. Count rules and test cases
5. Display test coverage statistics

### Manual Testing

#### Test Security Rules

```bash
# Run security analysis
cargo run -- analyze tests/xml/security_test.xml --rules tests/xml/xml_security.yaml

# Run with specific rule
cargo run -- analyze tests/xml/security_test.xml --rules tests/xml/xml_security.yaml --rule-id xml-xxe-001
```

#### Test Best Practices Rules

```bash
# Run best practices analysis
cargo run -- analyze tests/xml/best_practices_test.xml --rules tests/xml/best_practices.yaml

# Run with JSON output
cargo run -- analyze tests/xml/best_practices_test.xml --rules tests/xml/best_practices.yaml --json
```

#### Test on Sample Files

```bash
# Analyze Maven POM file
cargo run -- analyze tests/xml/pom.xml --rules tests/xml/xml_security.yaml

# Analyze SVG file
cargo run -- analyze tests/xml/image.svg --rules tests/xml/best_practices.yaml

# Analyze configuration file
cargo run -- analyze tests/xml/config.xml --rules tests/xml/xml_security.yaml
```

## 📊 Test Coverage

| Category | Rules | Test Cases | Coverage |
|----------|-------|------------|----------|
| Security | 15 | 60+ | High |
| Best Practices | 20 | 30+ | Medium |
| **Total** | **35** | **90+** | **High** |

## 🔍 Rule Categories

### By Severity

- **ERROR**: 8 rules (critical security issues)
- **WARNING**: 15 rules (important security/quality issues)
- **INFO**: 12 rules (code quality and style)

### By Confidence

- **HIGH**: 18 rules (high confidence detections)
- **MEDIUM**: 12 rules (medium confidence detections)
- **LOW**: 5 rules (low confidence, may have false positives)

## 📝 Example Detections

### XXE Vulnerability

```xml
<!-- VULNERABLE: External entity declaration -->
<!DOCTYPE foo [
  <!ENTITY xxe SYSTEM "file:///etc/passwd">
]>
<root>
  <data>&xxe;</data>
</root>
```

**Detection**: `xml-xxe-001` - XML External Entity (XXE) vulnerability detected

### Hardcoded Credentials

```xml
<!-- VULNERABLE: Hardcoded password -->
<database>
  <username>admin</username>
  <password>SuperSecret123!</password>
</database>
```

**Detection**: `xml-hardcoded-credentials-001` - Hardcoded password detected

### Weak Encryption

```xml
<!-- VULNERABLE: Weak algorithm -->
<encryption>
  <algorithm>DES</algorithm>
</encryption>
```

**Detection**: `xml-weak-encryption-001` - Weak encryption algorithm detected

## 🛠️ Adding New Tests

### Adding a Security Rule

1. Edit `xml_security.yaml`
2. Add new rule with unique ID
3. Specify pattern-regex for detection
4. Include metadata (CWE, OWASP, etc.)

### Adding Test Cases

1. Edit `security_test.xml` or `best_practices_test.xml`
2. Add vulnerable example with `<!-- ruleid: rule-id -->` comment
3. Add safe alternative for comparison
4. Include descriptive comments

### Example

```yaml
# In xml_security.yaml
rules:
  - id: xml-new-rule-001
    name: "New Security Check"
    description: "Detects new security issue"
    severity: ERROR
    languages: [xml]
    patterns:
      - pattern-regex: "<vulnerable>.*</vulnerable>"
    message: "Security issue detected"
```

```xml
<!-- In security_test.xml -->
<!-- VULNERABLE: New security issue -->
<!-- ruleid: xml-new-rule-001 -->
<vulnerable>bad content</vulnerable>

<!-- SAFE: Proper implementation -->
<safe>good content</safe>
```

## 📚 References

- [OWASP XML Security Cheat Sheet](https://cheatsheetseries.owasp.org/cheatsheets/XML_Security_Cheat_Sheet.html)
- [CWE-611: XXE](https://cwe.mitre.org/data/definitions/611.html)
- [CWE-798: Hardcoded Credentials](https://cwe.mitre.org/data/definitions/798.html)
- [XML Best Practices](https://www.w3.org/TR/xml/)

## 🤝 Contributing

To contribute new rules or test cases:

1. Follow the existing pattern structure
2. Include both vulnerable and safe examples
3. Add appropriate metadata (CWE, OWASP)
4. Test your rules with `run_xml_tests.py`
5. Update this README with new rule information

## ✅ Validation Checklist

- [ ] All XML files are well-formed
- [ ] All YAML files have valid syntax
- [ ] Each rule has corresponding test cases
- [ ] Test cases include both vulnerable and safe examples
- [ ] Rules include proper metadata (severity, CWE, OWASP)
- [ ] README is updated with new rules
- [ ] Test runner script passes all checks

## 📄 License

These test cases are part of the cr-semservice project and follow the same license.

