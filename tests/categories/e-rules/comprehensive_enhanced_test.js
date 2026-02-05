// Comprehensive test file for enhanced pattern features in JavaScript

// Test dangerous eval comprehensive
function processUserInput(userCode) {
    // Should match - eval with string concatenation outside sandbox/try-catch
    eval("var result = " + userCode + ";");
    
    // Should match - another concatenation pattern
    return eval("process_" + operation + "(" + data + ")");
}

function sandboxedExecution(code) {
    // Should NOT match - inside sandbox function
    function sandboxHelper() {
        eval(code);
    }
    sandboxHelper();
}

function sandboxProcessor(input) {
    // Should NOT match - inside sandbox function
    eval("processInput(" + input + ")");
}

function safeEvaluation(code) {
    try {
        // Should NOT match - inside try-catch block
        eval("var temp = " + code + ";");
    } catch (error) {
        console.error("Eval failed:", error);
    }
}

function testEvalFunction() {
    // Should NOT match - test function excluded by pattern-not-regex
    eval("var testVar = " + testData + ";");
}

function mockEvaluation() {
    // Should NOT match - mock function excluded by pattern-not-regex
    eval("var mockResult = " + mockData + ";");
}

// Complex nested scenarios
function complexProcessing() {
    if (isProduction) {
        // Should match - eval with concatenation outside safety blocks
        eval("config." + property + " = " + value + ";");
        
        try {
            // Should NOT match - inside try-catch
            eval("backup." + key + " = " + backup_value + ";");
        } catch (e) {
            handleError(e);
        }
    }
}

// Edge cases
function dynamicExecution() {
    const code = "function_" + type + "(" + params + ")";
    
    // Should match - eval with concatenated string
    eval(code);
}

function conditionalEval() {
    if (allowDynamicCode) {
        // Should match - eval with concatenation
        eval("handler_" + eventType + "(event)");
    }
}

// Safe alternatives (should not match)
function safeAlternatives() {
    // These should not match as they don't use string concatenation
    eval("staticCode()");
    eval(precomputedCode);
    
    // Function calls that look similar but aren't eval
    evaluate("some_" + "code");
    execute("command_" + "args");
}
