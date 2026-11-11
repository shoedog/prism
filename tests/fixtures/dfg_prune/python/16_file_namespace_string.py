import sys


def f():
    x = 1
    x = sys.modules["builtins"].eval("x + 1")
    return x
