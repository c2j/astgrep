public class TestClass {
    public void badLoggingExamples() {
        // These should be detected by the rule
        System.out.println("Debug: Processing user data");
        System.out.print("Status: ");
        System.err.println("Error occurred: " + getMessage());
        System.err.print("Warning: ");
        //System.err.print("Warning: ");
        
        String userId = "12345";
        System.out.println("User ID: " + userId);
    }
    
    public void goodLoggingExamples() {
        // These are proper logging practices
        Logger logger = LoggerFactory.getLogger(TestClass.class);
        
        logger.info("Processing user data");
        logger.debug("Debug information");
        logger.error("Error occurred", exception);
    }
    
    private String getMessage() {
        return "test message";
    }
}