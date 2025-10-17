import org.slf4j.Logger;
import org.slf4j.LoggerFactory;

public class LoggingExample {
    private static final Logger logger = LoggerFactory.getLogger(LoggingExample.class);
    
    public void badLogging() {
        try {
            // 一些可能抛异常的代码
            int result = 10 / 0;
        } catch (Exception e) {
            // ruleid: CS-EH-08
            e.printStackTrace();
            
            // ruleid: CS-EH-08
            System.out.println("Error occurred: " + e.getMessage());
            
            // ruleid: CS-EH-08
            System.err.println("Error: " + e.getMessage());
            
            // ruleid: CS-EH-08
            System.out.print("Debug info");
            
            // ruleid: CS-EH-08
            System.err.printf("Error code: %d", 500);
        }
        
        // ruleid: CS-EH-08
        System.out.println("Application started");
        
        // ruleid: CS-EH-08
        System.err.format("Warning: %s", "something wrong");
    }
    
    public void goodLogging() {
        try {
            // 一些可能抛异常的代码
            int result = 10 / 0;
        } catch (Exception e) {
            // ok: CS-EH-08
            logger.error("Error occurred", e);
            
            // ok: CS-EH-08
            logger.warn("Warning: {}", e.getMessage());
        }
        
        // ok: CS-EH-08
        logger.info("Application started");
        
        // ok: CS-EH-08
        logger.debug("Debug info: {}", "some value");
    }
}
