// ruleid: CS-OT-04
import java.util.*;

// ruleid: CS-OT-04
import java.io.*;

// ruleid: CS-OT-04
import javax.servlet.*;

// ruleid: CS-OT-04
import org.springframework.web.bind.annotation.*;

// ok: CS-OT-04
import java.util.List;
import java.util.ArrayList;
import java.util.HashMap;
import java.io.IOException;
import java.io.FileInputStream;
import javax.servlet.http.HttpServletRequest;
import org.springframework.web.bind.annotation.RestController;
import org.springframework.web.bind.annotation.GetMapping;

public class ImportExample {
    private List<String> list = new ArrayList<>();
    private HashMap<String, Object> map = new HashMap<>();
    
    public void processFile() throws IOException {
        FileInputStream fis = new FileInputStream("test.txt");
        // 处理文件
    }
}
