import java.sql.*;

public class TestSecurity {
    private String password = "hardcoded123";
    
    public void vulnerableQuery(String userInput) {
        try {
            Connection conn = DriverManager.getConnection("jdbc:mysql://localhost/test");
            Statement stmt = conn.createStatement();
            String query = "SELECT * FROM users WHERE id = " + userInput;
            ResultSet rs = stmt.executeQuery(query);
            
            while (rs.next()) {
                System.out.println(rs.getString("name"));
            }
        } catch (SQLException e) {
            e.printStackTrace();
        }
    }
    
    public void anotherMethod() {
        String adminPassword = "admin123";
        System.out.println("Admin password is: " + adminPassword);
    }
}
