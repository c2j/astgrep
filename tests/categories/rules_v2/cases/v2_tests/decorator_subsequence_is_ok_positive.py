# @rule decorator-subsequence-is-ok
# @desc decorator_subsequence_is_ok v2 syntax test
# @expect MATCH

# ruleid: decorator-subsequence-is-ok
@first("syn")
@second("ack")
@third("third")
def func1():
    pass
