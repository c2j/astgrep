# @rule explain-basic
# @desc basic explanation feature test
# @expect MATCH

def foo():
    foo()
    foo(bar())
    bar()
