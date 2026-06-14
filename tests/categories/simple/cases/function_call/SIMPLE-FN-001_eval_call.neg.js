// @rule SIMPLE-FN-001
// @desc Non-eval function calls should NOT trigger
// @expect NO_MATCH
console.log("safe");
setTimeout(fn, 100);
function safe() { return 42; }
