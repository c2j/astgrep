// Test JavaScript file for CR GUI
function unsafeFunction() {
    // SQL injection vulnerability
    var query = "SELECT * FROM users WHERE id = " + userId;
    
    // XSS vulnerability
    document.getElementById("output").innerHTML = userInput;
    
    // Command injection
    exec("ls " + userInput);
    
    // Hardcoded credentials
    var password = "admin123";
    var apiKey = "sk-1234567890abcdef";
    
    // Insecure random
    var token = Math.random().toString(36);
    
    return query;
}

function betterFunction() {
    // This is a safer implementation
    const query = "SELECT * FROM users WHERE id = ?";
    
    // Use textContent instead of innerHTML
    document.getElementById("output").textContent = userInput;
    
    // Validate input before using
    if (!/^[a-zA-Z0-9]+$/.test(userInput)) {
        throw new Error("Invalid input");
    }
    
    return query;
}
