// @rule rule_template_id
# @desc metavar_cond_octal v2 syntax test
// @expect MATCH

package Foo

func foo() {
     os.Mkdir("foo", 0400)
     os.Mkdir("foo", 0600)
     //ruleid: rule_template_id
     os.Mkdir("foo", 0666)
}
