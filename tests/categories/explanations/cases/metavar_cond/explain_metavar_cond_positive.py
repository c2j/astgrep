# @rule explain-metavar_cond
# @desc metavar_cond explanation feature test
# @expect MATCH

def foo():
    foo(10)
    foo(30)
    foo(5)
