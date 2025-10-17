// Test JavaScript file with potential security issues
function unsafeFunction(userInput) {
    // SQL injection vulnerability
    const query = "SELECT * FROM users WHERE id = " + userInput;
    
    // XSS vulnerability
    document.getElementById("output").innerHTML = userInput;
    
    // Command injection vulnerability
    const exec = require('child_process').exec;
    exec("ls " + userInput, (error, stdout, stderr) => {
        console.log(stdout);
    });
    
    return query;
}

// Path traversal vulnerability
function readFile(filename) {
    const fs = require('fs');
    return fs.readFileSync(filename, 'utf8');
}

// Insecure random number generation
function generateToken() {
    return Math.random().toString(36).substring(2);
}

// Hardcoded credentials
const API_KEY = "sk-1234567890abcdef";
const PASSWORD = "admin123";

// Unsafe eval usage
function processUserCode(code) {
    return eval(code);
}
