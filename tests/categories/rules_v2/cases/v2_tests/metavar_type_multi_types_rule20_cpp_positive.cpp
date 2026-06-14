// @rule match-multiple-metavar-type-rule20
# @desc metavar_type_multi_types_rule20_cpp v2 syntax test
// @expect MATCH

#include <fstream>

using namespace std;

void test_001() {
    ifstream in;
    // ruleid: match-multiple-metavar-type-rule20
    in.get(str, 2);

    mystream my;
    // ruleid: match-multiple-metavar-type-rule20
    my.get(str, 2);

    yourstream your;
    // ok: match-multiple-metavar-type-rule20
    your.get(str, 2);
}
