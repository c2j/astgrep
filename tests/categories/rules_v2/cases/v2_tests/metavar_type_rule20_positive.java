// @rule no-string-eqeq
# @desc metavar_type_rule20 v2 syntax test
// @expect MATCH

public class Example {
    public int foo(String a, int b) {
        // ruleid: no-string-eqeq
        if (a == "hello") return 1;
        // ruleid: no-string-eqeq
        if ("hello" == a) return 2;
        // ok: no-string-eqeq
        if (b == 2) return -1;
        // ok: no-string-eqeq
        if (null == "hello") return 12;
        // ok: no-string-eqeq
        if ("hello" == null) return 0;
    }
}
