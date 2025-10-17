import java.util.List;
import java.util.Arrays;

public class StringConcatenationExample {
    
    public void badStringConcatenation() {
        List<String> items = Arrays.asList("a", "b", "c", "d", "e");
        
        // ruleid: CS-PF-01
        String result1 = "";
        for (String item : items) {
            result1 = result1 + item + ",";
        }
        
        // ruleid: CS-PF-01
        String result2 = "";
        for (int i = 0; i < items.size(); i++) {
            result2 += items.get(i) + ",";
        }
        
        // ruleid: CS-PF-01
        String result3 = "";
        int i = 0;
        while (i < items.size()) {
            result3 = result3 + items.get(i);
            i++;
        }
        
        // ruleid: CS-PF-01
        String result4 = "";
        int j = 0;
        do {
            result4 += items.get(j);
            j++;
        } while (j < items.size());
    }
    
    public void goodStringConcatenation() {
        List<String> items = Arrays.asList("a", "b", "c", "d", "e");
        
        // ok: CS-PF-01 - 使用 StringBuilder
        StringBuilder sb1 = new StringBuilder();
        for (String item : items) {
            sb1.append(item).append(",");
        }
        String result1 = sb1.toString();
        
        // ok: CS-PF-01 - 使用 StringBuffer (线程安全)
        StringBuffer sb2 = new StringBuffer();
        for (int i = 0; i < items.size(); i++) {
            sb2.append(items.get(i)).append(",");
        }
        String result2 = sb2.toString();
        
        // ok: CS-PF-01 - 使用 String.join
        String result3 = String.join(",", items);
        
        // ok: CS-PF-01 - 使用 Stream API
        String result4 = items.stream()
                .collect(java.util.stream.Collectors.joining(","));
        
        // ok: CS-PF-01 - 循环外的字符串拼接
        String prefix = "prefix";
        String suffix = "suffix";
        String singleResult = prefix + "_" + suffix;
    }
    
    public void mixedExample() {
        List<String> items = Arrays.asList("item1", "item2", "item3");
        
        // ok: CS-PF-01 - 正确使用 StringBuilder
        StringBuilder sb = new StringBuilder();
        sb.append("Start: ");
        for (String item : items) {
            sb.append("[").append(item).append("]");
        }
        sb.append(" :End");
        String result = sb.toString();
        
        System.out.println(result);
    }
}
