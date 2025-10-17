public class MethodParametersExample {
    
    // ruleid: CS-RD-09
    public void badMethodTooManyParams(String name, int age, String email, 
                                      String phone, String address, String city, 
                                      String country, String zipCode) {
        // 处理逻辑
    }
    
    // ruleid: CS-RD-09
    public User createUser(String firstName, String lastName, String email, 
                          String phone, int age, String address, String city, 
                          String country, String zipCode, boolean isActive) {
        return new User();
    }
    
    // ok: CS-RD-09
    public void goodMethodFewParams(String name, int age, String email) {
        // 处理逻辑
    }
    
    // ok: CS-RD-09 - 使用对象传递参数
    public void goodMethodWithObject(UserRequest request) {
        // 处理逻辑
    }
    
    // ok: CS-RD-09
    public User createUserWithObject(UserCreateRequest request) {
        return new User();
    }
    
    // 参数对象
    public static class UserRequest {
        private String name;
        private int age;
        private String email;
        private String phone;
        private String address;
        private String city;
        private String country;
        private String zipCode;
        // getters and setters
    }
    
    public static class UserCreateRequest {
        private String firstName;
        private String lastName;
        private String email;
        private String phone;
        private int age;
        private String address;
        private String city;
        private String country;
        private String zipCode;
        private boolean isActive;
        // getters and setters
    }
    
    public static class User {
        // User implementation
    }
}
