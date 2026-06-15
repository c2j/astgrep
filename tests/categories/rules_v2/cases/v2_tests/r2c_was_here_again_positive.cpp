// @rule here
# @desc r2c_was_here_again v2 syntax test
// @expect MATCH

class A {
    void main() {
	// ruleid: here
	stuff(1);
	not_stuff();
	// ruleid: here
	r_2_c_was_here();
	// proruleid: here
	r_2_c_pro_was_here();
    }
}
