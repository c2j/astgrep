# XML Tests - Quick Start Guide

## 🚀 Quick Start (30 seconds)

### 1. Validate Test Suite

```bash
cd tests/xml
python3 run_xml_tests.py
```

Expected output:
```
✓ All XML tests passed!
✓ 6 XML files validated
✓ 2 YAML rule files validated
✓ 35 security rules defined
✓ 54 test cases created
```

### 2. Run Security Analysis

```bash
# From project root
cargo run -- analyze tests/xml/security_test.xml --rules tests/xml/xml_security.yaml
```

### 3. Run Best Practices Check

```bash
cargo run -- analyze tests/xml/best_practices_test.xml --rules tests/xml/best_practices.yaml
```

## 📋 What's Included

### Security Rules (15 rules)

| Rule ID | Description | Severity |
|---------|-------------|----------|
| xml-xxe-001 | XXE - External Entity | ERROR |
| xml-xxe-002 | XXE - Parameter Entity | ERROR |
| xml-hardcoded-credentials-001 | Hardcoded Passwords | ERROR |
| xml-hardcoded-credentials-002 | Hardcoded API Keys | ERROR |
| xml-weak-encryption-001 | Weak Encryption | WARNING |
| xml-insecure-protocol-001 | Insecure HTTP | WARNING |
| xml-debug-enabled-001 | Debug Mode Enabled | WARNING |
| xml-ssl-disabled-001 | SSL/TLS Disabled | ERROR |
| ... | (7 more rules) | ... |

### Best Practice Rules (20 rules)

| Rule ID | Description | Severity |
|---------|-------------|----------|
| xml-best-practice-001 | Missing XML Declaration | INFO |
| xml-empty-element-001 | Empty Element Not Self-Closing | INFO |
| xml-attribute-quotes-001 | Attribute Without Quotes | WARNING |
| xml-comment-best-practice-001 | Comment Contains Double Hyphen | WARNING |
| xml-boolean-values-001 | Non-Standard Boolean Values | INFO |
| ... | (15 more rules) | ... |

## 🎯 Common Use Cases

### Check for XXE Vulnerabilities

```bash
cargo run -- analyze your-file.xml --rules tests/xml/xml_security.yaml --rule-id xml-xxe-001
```

### Find Hardcoded Credentials

```bash
cargo run -- analyze config.xml --rules tests/xml/xml_security.yaml --rule-id xml-hardcoded-credentials-001
```

### Validate XML Best Practices

```bash
cargo run -- analyze your-file.xml --rules tests/xml/best_practices.yaml
```

### Get JSON Output

```bash
cargo run -- analyze your-file.xml --rules tests/xml/xml_security.yaml --json > results.json
```

## 📁 File Overview

```
tests/xml/
├── xml_security.yaml              # 15 security rules
├── security_test.xml              # 49 security test cases
├── best_practices.yaml            # 20 best practice rules
├── best_practices_test.xml        # 5 best practice test cases
├── run_xml_tests.py              # Automated test runner
├── README.md                      # Full documentation
├── TEST_SUMMARY.md               # Detailed summary
├── QUICK_START.md                # This file
└── [sample files]                # XML examples
```

## 🔍 Example Detections

### XXE Vulnerability

**Vulnerable Code:**
```xml
<!DOCTYPE foo [
  <!ENTITY xxe SYSTEM "file:///etc/passwd">
]>
<root><data>&xxe;</data></root>
```

**Detection:** `xml-xxe-001` - XML External Entity (XXE) vulnerability detected

### Hardcoded Password

**Vulnerable Code:**
```xml
<database>
  <password>SuperSecret123!</password>
</database>
```

**Detection:** `xml-hardcoded-credentials-001` - Hardcoded password detected

### Weak Encryption

**Vulnerable Code:**
```xml
<encryption>
  <algorithm>DES</algorithm>
</encryption>
```

**Detection:** `xml-weak-encryption-001` - Weak encryption algorithm detected

## 🛠️ Troubleshooting

### Python Script Fails

**Issue:** `python3: command not found`

**Solution:** Install Python 3 or use `python` instead of `python3`

### xmllint Not Found

**Issue:** `xmllint not found - skipping XML syntax validation`

**Solution:** This is just a warning. Install libxml2 if you want syntax validation:
```bash
# macOS
brew install libxml2

# Ubuntu/Debian
sudo apt-get install libxml2-utils
```

### PyYAML Not Installed

**Issue:** `PyYAML not installed - skipping YAML validation`

**Solution:** Install PyYAML (optional):
```bash
pip3 install pyyaml
```

## 📊 Test Statistics

- **Total Rules**: 35
- **Security Rules**: 15
- **Best Practice Rules**: 20
- **Test Cases**: 54
- **Sample Files**: 4
- **Coverage**: High

## 🎓 Learning Resources

### Security

- [OWASP XML Security Cheat Sheet](https://cheatsheetseries.owasp.org/cheatsheets/XML_Security_Cheat_Sheet.html)
- [CWE-611: XXE](https://cwe.mitre.org/data/definitions/611.html)
- [XML Security Best Practices](https://www.w3.org/TR/xml/)

### Best Practices

- [XML 1.0 Specification](https://www.w3.org/TR/xml/)
- [XML Naming Conventions](https://www.w3.org/TR/xml-names/)
- [XML Schema Best Practices](https://www.w3.org/TR/xmlschema-0/)

## ✅ Quick Checklist

Before committing XML files:

- [ ] Run `python3 tests/xml/run_xml_tests.py`
- [ ] Check for XXE vulnerabilities
- [ ] Verify no hardcoded credentials
- [ ] Ensure HTTPS is used (not HTTP)
- [ ] Check encryption algorithms
- [ ] Validate XML syntax
- [ ] Review security warnings

## 🤝 Need Help?

1. Check [README.md](README.md) for detailed documentation
2. Review [TEST_SUMMARY.md](TEST_SUMMARY.md) for complete coverage
3. Look at test cases in `security_test.xml` and `best_practices_test.xml`
4. Run `python3 run_xml_tests.py` for validation

## 📝 Quick Commands Reference

```bash
# Validate test suite
python3 tests/xml/run_xml_tests.py

# Run all security checks
cargo run -- analyze tests/xml/security_test.xml --rules tests/xml/xml_security.yaml

# Run all best practice checks
cargo run -- analyze tests/xml/best_practices_test.xml --rules tests/xml/best_practices.yaml

# Check specific file
cargo run -- analyze your-file.xml --rules tests/xml/xml_security.yaml

# Get JSON output
cargo run -- analyze your-file.xml --rules tests/xml/xml_security.yaml --json

# Check specific rule
cargo run -- analyze your-file.xml --rules tests/xml/xml_security.yaml --rule-id xml-xxe-001
```

---

**Ready to start?** Run `python3 tests/xml/run_xml_tests.py` now! 🚀

