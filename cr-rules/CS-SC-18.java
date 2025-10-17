import org.springframework.beans.factory.annotation.Value;
import org.springframework.core.env.Environment;

public class PasswordHardcodingExample {
    
    @Value("${database.password}")
    private String dbPassword;
    
    private Environment environment;
    
    public void badPasswordHardcoding() {
        // ruleid: CS-SC-18
        String password = "123456";
        
        // ruleid: CS-SC-18
        String dbPassword = "admin123";
        
        // ruleid: CS-SC-18
        String apiSecret = "secret_key_123";
        
        // ruleid: CS-SC-18
        String accessToken = "abc123def456";
        
        // ruleid: CS-SC-18
        String credential = "user:password123";
        
        // ruleid: CS-SC-18 - 空密码
        String emptyPassword = "";
        
        // ruleid: CS-SC-18 - null密码
        String nullPassword = null;
        
        // 使用硬编码密码连接数据库
        connectToDatabase("localhost", "admin", password);
    }
    
    public void goodPasswordHandling() {
        // ok: CS-SC-18 - 从配置文件读取
        String password = environment.getProperty("database.password");
        
        // ok: CS-SC-18 - 使用注入的值
        String dbPass = this.dbPassword;
        
        // ok: CS-SC-18 - 从系统属性读取
        String apiSecret = System.getProperty("api.secret");
        
        // ok: CS-SC-18 - 从环境变量读取
        String token = System.getenv("ACCESS_TOKEN");
        
        // ok: CS-SC-18 - 从用户输入获取
        String userPassword = getUserInputPassword();
        
        // ok: CS-SC-18 - 从密钥管理系统获取
        String credential = getCredentialFromVault("db-credential");
        
        // 使用安全方式获取的密码
        connectToDatabase("localhost", "admin", password);
    }
    
    private String getUserInputPassword() {
        // 从用户输入获取密码的逻辑
        return "user_input_password";
    }
    
    private String getCredentialFromVault(String credentialName) {
        // 从密钥管理系统获取凭证的逻辑
        return "vault_credential";
    }
    
    private void connectToDatabase(String host, String username, String password) {
        // 数据库连接逻辑
    }
}
