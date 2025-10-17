import java.util.Random;
import java.security.SecureRandom;

public class RandomNumberExample {
    
    public void badRandomUsage() {
        // ruleid: CS-SC-17
        Random random1 = new Random();
        int value1 = random1.nextInt(100);
        
        // ruleid: CS-SC-17
        Random random2 = new Random(System.currentTimeMillis());
        int value2 = random2.nextInt(100);
        
        // ruleid: CS-SC-17
        java.util.Random random3 = new java.util.Random();
        int value3 = random3.nextInt(100);
        
        // ruleid: CS-SC-17
        double value4 = Math.random();
        
        // 生成随机密码或令牌时使用不安全的随机数
        String token = generateToken(random1);
    }
    
    public void goodRandomUsage() {
        // ok: CS-SC-17
        SecureRandom secureRandom1 = new SecureRandom();
        int value1 = secureRandom1.nextInt(100);
        
        // ok: CS-SC-17
        java.security.SecureRandom secureRandom2 = new java.security.SecureRandom();
        int value2 = secureRandom2.nextInt(100);
        
        // ok: CS-SC-17
        SecureRandom secureRandom3 = SecureRandom.getInstance("SHA1PRNG");
        byte[] randomBytes = new byte[16];
        secureRandom3.nextBytes(randomBytes);
        
        // 生成安全的随机密码或令牌
        String token = generateSecureToken(secureRandom1);
    }
    
    private String generateToken(Random random) {
        // 不安全的令牌生成
        return String.valueOf(random.nextLong());
    }
    
    private String generateSecureToken(SecureRandom secureRandom) {
        // 安全的令牌生成
        byte[] bytes = new byte[32];
        secureRandom.nextBytes(bytes);
        return java.util.Base64.getEncoder().encodeToString(bytes);
    }
}
