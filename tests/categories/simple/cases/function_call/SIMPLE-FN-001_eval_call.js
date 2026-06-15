// @rule SIMPLE-FN-001
// @desc eval() calls should be detected
// @expect MATCH
eval("some code");
eval(userInput);
