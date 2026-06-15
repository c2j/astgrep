# @rule explain-negation
# @desc negation explanation feature test
# @expect MATCH

def foo():
    foo()
    foo(bar())
    bar()
