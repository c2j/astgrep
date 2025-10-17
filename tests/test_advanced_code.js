// Test cases for advanced metavariable features

// Test metavariable-name: should match fs.require
const fs = require('fs');
fs.require('./config');

// Test metavariable-name: should NOT match other modules
const path = require('path');
path.require('./config');

// Test metavariable-analysis entropy: should match high entropy strings
const secret1 = "abc123XYZ"; // Medium entropy
const secret2 = "a1b2c3d4e5"; // Higher entropy
const secret3 = "password"; // Low entropy

// Test metavariable-analysis type: function parameters
function processString(data) { return data; }
function processNumber(count) { return count; }
function processNull(empty) { return empty; }

// Test metavariable-analysis complexity: simple vs complex functions
function simpleFunc() {
    console.log("simple");
}

function complexFunc() {
    if (condition1) {
        if (condition2) {
            for (let i = 0; i < 10; i++) {
                if (condition3) {
                    console.log("complex");
                }
            }
        }
    }
}

// Test Python expression: long vs short values
const shortVar = "short";
const longVariable = "this is a very long string that should trigger the rule";

// Test pattern-all: dangerous function definitions and calls
function eval(code) { return code; }
eval("malicious code");

function exec(command) { return command; }
exec("rm -rf /");

// Test pattern-any: various eval patterns
eval("dangerous");
Function("return 1");
new Function("alert('xss')");

// Test multiple focus-metavariable
function authenticate(password, secret) {
    return password === secret;
}

function login(username, key) {
    return username && key;
}
