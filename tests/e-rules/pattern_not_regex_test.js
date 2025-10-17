// Test file for pattern-not-regex functionality in JavaScript

// Function call tests
function testFunctionCalls() {
    // Should match - basic function calls
    alert("message");
    parseInt("42");
    parseFloat("3.14");
    
    // Should NOT match - method calls (excluded by pattern-not-regex)
    obj.method();
    array.push(item);
    string.toLowerCase();
    
    // Should NOT match - constructor calls (excluded by pattern-not-regex)
    new Date();
    new Array(10);
    new Object();
    
    // Should match - more basic function calls
    setTimeout(callback, 1000);
    clearInterval(intervalId);
}

// Console tests
function testConsole() {
    // Should match - console.log and other console methods
    console.log("debug message");
    console.info("information");
    console.debug("debug info");
    
    // Should NOT match - excluded console methods
    console.error("error message");
    console.warn("warning message");
}

// URL tests
const urls = {
    // Should match - HTTP URLs
    insecure: "http://example.com",
    api: "http://api.service.com/endpoint",
    local: "http://localhost:3000",
    
    // Should NOT match - HTTPS URLs
    secure: "https://secure.example.com",
    secureApi: "https://api.secure.com/endpoint"
};

// Mixed examples
function processData() {
    // Should match
    validateInput(data);
    
    // Should NOT match
    data.validate();
    new Validator(rules);
    
    console.log("Processing complete");  // Should match
    console.error("Processing failed");  // Should NOT match
}
