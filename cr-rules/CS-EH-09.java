public class ExceptionHandlingExample {
    
    public void badExceptionHandling() {
        try {
            String str = null;
            int length = str.length(); // 可能抛出 NPE
        } catch (NullPointerException e) {
            // ruleid: CS-EH-09
            System.out.println("Caught NPE");
        }
        
        try {
            int result = 10 / 0; // 可能抛出 ArithmeticException
        } catch (RuntimeException e) {
            // ruleid: CS-EH-09
            System.out.println("Caught RuntimeException");
        }
        
        try {
            processData(null);
        } catch (NullPointerException | IllegalArgumentException e) {
            // ruleid: CS-EH-09
            System.out.println("Caught exception");
        }
        
        try {
            riskyOperation();
        } catch (RuntimeException | IOException e) {
            // ruleid: CS-EH-09
            System.out.println("Caught exception");
        }
    }
    
    public void goodExceptionHandling() {
        // ok: CS-EH-09 - 预防性检查而不是捕获 NPE
        String str = getString();
        if (str != null) {
            int length = str.length();
        }
        
        // ok: CS-EH-09 - 捕获具体的异常类型
        try {
            int result = 10 / getDivisor();
        } catch (ArithmeticException e) {
            System.out.println("Division by zero");
        }
        
        // ok: CS-EH-09 - 捕获具体的异常
        try {
            processData(getData());
        } catch (IllegalArgumentException e) {
            System.out.println("Invalid argument");
        } catch (IllegalStateException e) {
            System.out.println("Invalid state");
        }
        
        // ok: CS-EH-09 - 捕获具体的异常
        try {
            riskyOperation();
        } catch (IOException e) {
            System.out.println("IO error");
        } catch (SQLException e) {
            System.out.println("Database error");
        }
    }
    
    private String getString() {
        return "test";
    }
    
    private int getDivisor() {
        return 2;
    }
    
    private Object getData() {
        return new Object();
    }
    
    private void processData(Object data) {
        if (data == null) {
            throw new IllegalArgumentException("Data cannot be null");
        }
        // 处理数据
    }
    
    private void riskyOperation() throws IOException, SQLException {
        // 可能抛出异常的操作
    }
}
