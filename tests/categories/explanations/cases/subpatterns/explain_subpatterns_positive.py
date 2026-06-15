# @rule explain-subpatterns
# @desc subpatterns explanation feature test
# @expect MATCH

def test():
    foo(1)
    stuff()
    bar(1)

def test2():
    foo(1)
    stuff()
    bar(2)
