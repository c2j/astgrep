// Simple Java test for basic pattern matching

public class TestClass {
    
    public void testMethod() {
        // String literals
        String message = "hello world";
        String greeting = "hello world";
        
        // Numeric literals
        int x = 42;
        double y = 3.14;
        String z = "42"; // This is a string, not a number
        
        // Method calls
        System.out.println("test");
        System.out.println(message);
        
        // More examples
        processData("input");
        processData(42);
    }
    
    private void processData(String input) {
        System.out.println("Processing: " + input);
    }
    
    private void processData(int value) {
        System.out.println("Processing: " + value);
    }
}
