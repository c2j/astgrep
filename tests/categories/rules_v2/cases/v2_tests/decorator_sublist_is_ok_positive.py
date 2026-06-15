# @rule decorator-sublist-is-ok
# @desc decorator_sublist_is_ok v2 syntax test
# @expect MATCH

# ruleid: decorator-sublist-is-ok
@first("syn")
@second("ack")
@third("test")
def func1():
    pass
