# @rule new-syntax
# @desc new_syntax v2 syntax test
# @expect MATCH


def bar():
    foo(1, 2, 4)
    # ruleid: new-syntax
    foo(1, 2, 3)

foo(1, 2, 3)