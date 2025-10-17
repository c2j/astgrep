// Test JavaScript security issues

function vulnerableFunction(userInput) {
    // Dangerous eval usage
    eval("var result = " + userInput);
    
    // XSS vulnerability
    document.getElementById("content").innerHTML = userInput;
    
    return result;
}

function anotherVulnerableFunction() {
    var code = "alert('hello')";
    eval(code);
}
