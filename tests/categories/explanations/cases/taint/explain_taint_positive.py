# @rule explain-taint
# @desc taint explanation feature test
# @expect MATCH

def foo():
    a = source()
    b = a
    c = b
    sink(c)

def foo2():
    a = source()
    b = a
    c = b
    other(c)

def foo2():
    a = other()
    b = a
    c = b
    sink(c)
