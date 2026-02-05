// This should match
eval("some code");

// This should also match
eval(userInput);

// This should not match
evaluate("something");

function test() {
    // This should match
    eval(data);
}
