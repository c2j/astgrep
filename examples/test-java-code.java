// Test Java code for System.out logging rule
public class TestClass {
    
    public void badLoggingExamples() {
        // These should be detected by the rule
        System.out.println("Debug: Processing user data");
        System.out.print("Status: ");
        System.err.println("Error occurred: " + getMessage());
        System.err.print("Warning: ");
        
        String userId = "12345";
        System.out.println("User ID: " + userId);
        
        // More complex examples
        if (isDebugMode()) {
            System.out.println("Debug mode is enabled");
        }
        
        for (int i = 0; i < 10; i++) {
            System.out.print(i + " ");
        }
        System.out.println();
    }
    
    public void goodLoggingExamples() {
        // These are proper logging practices (should not be detected)
        Logger logger = LoggerFactory.getLogger(TestClass.class);
        
        logger.info("Processing user data");
        logger.debug("Debug information");
        logger.error("Error occurred", exception);
        logger.warn("Warning message");
        
        // Using SLF4J parameterized logging
        String userId = "12345";
        logger.info("User ID: {}", userId);
    }
    
    private boolean isDebugMode() {
        return true;
    }
    
    private String getMessage() {
        return "test message";
    }
}
