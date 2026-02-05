// Test file for focus-metavariable functionality in JavaScript

// Test focusing on multiple variables
function processUserData(userInfo, data, configSettings) {
    // Should focus on 'userInfo' and 'configSettings' parameters
    return handleData(userInfo, data, configSettings);
}

function authenticateUser(authToken, payload, settingsObj) {
    // Should focus on 'authToken' and 'settingsObj' parameters
    return validateAuth(authToken, payload, settingsObj);
}

function setupSystem(userCredentials, middleware, systemConfig) {
    // Should focus on 'userCredentials' and 'systemConfig' parameters
    return initializeSystem(userCredentials, middleware, systemConfig);
}

function regularFunction(param1, param2, param3) {
    // Should NOT match - parameters don't match the regex patterns
    return processData(param1, param2, param3);
}

// Test with different parameter patterns
function handleRequest(userData, request, appSettings) {
    // Should focus on 'userData' and 'appSettings'
    const result = processRequest(userData, request, appSettings);
    return result;
}

function manageAuth(authInfo, session, configData) {
    // Should focus on 'authInfo' and 'configData'
    return validateSession(authInfo, session, configData);
}

// Edge cases
function singleMatch(userAccount, data, other) {
    // Should focus only on 'userAccount' (matches user pattern)
    return process(userAccount, data, other);
}

function noMatch(param1, param2, param3) {
    // Should NOT match - no parameters match the patterns
    return execute(param1, param2, param3);
}

// Complex nested example
function complexFunction(userProfile, requestData, systemSettings) {
    // Should focus on 'userProfile' and 'systemSettings'
    if (userProfile.isValid) {
        return processWithSettings(userProfile, requestData, systemSettings);
    }
    return null;
}
