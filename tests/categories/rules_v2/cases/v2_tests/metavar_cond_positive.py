# @rule rule_template_id
# @desc metavar_cond v2 syntax test
# @expect MATCH

def test():
    #ruleid: rule_template_id
    foo("a_bar_variable")

    foo("a_ba_variable")
