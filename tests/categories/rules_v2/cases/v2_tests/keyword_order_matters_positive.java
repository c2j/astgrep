// @rule keyword-order-matters
# @desc keyword_order_matters v2 syntax test
// @expect MATCH

public class test {
    // ruleid: keyword-order-matters
    private static final int a;
    // ruleid: keyword-order-matters
    private final static int b;
}
