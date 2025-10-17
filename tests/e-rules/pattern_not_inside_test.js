// Test file for pattern-not-inside functionality in JavaScript

function processData(input) {
    // Should match - eval outside try-catch
    eval("console.log('test')");
    eval(input);
    
    // Safe eval with try-catch
    try {
        // Should NOT match - eval inside try-catch
        eval(dynamicCode);
        eval("result = " + calculation);
    } catch (error) {
        console.error("Eval failed:", error);
    }
    
    // Should match - eval outside try-catch again
    return eval("result");
}

class DataProcessor {
    process(data) {
        // Should match - eval outside try-catch
        eval(data);
        
        try {
            // Should NOT match - eval inside try-catch
            eval("this.result = " + data);
        } catch (e) {
            this.handleError(e);
        }
    }
}

// Should match - eval outside try-catch
eval("globalVariable = 42");

// Complex nested case
function complexFunction() {
    if (condition) {
        // Should match - eval outside try-catch
        eval(userInput);
        
        try {
            // Should NOT match - eval inside try-catch
            eval("processedData = transform(" + userInput + ")");
        } catch (error) {
            console.log("Error:", error);
        }
    }
}
