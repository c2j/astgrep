// @rule JAVA-RED-001
// @desc short method with only 10 lines, well under 80 threshold
// @expect NO_MATCH
public class MyService {
    public void process() {
        String step1 = "init";
        String step2 = "load";
        String step3 = "validate";
        String step4 = "transform";
        String step5 = "output";
    }
}
