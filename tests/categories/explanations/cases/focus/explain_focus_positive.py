# @rule explain-focus
# @desc focus explanation feature test
# @expect MATCH

def foo():
    foo(1, 2, 3)
    foo(2, 2, 2)
    foo(0, 1)
