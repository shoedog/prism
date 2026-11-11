import __builtin__


def f():
    x = 1
    x = __builtin__.eval("x + 1")
    return x
