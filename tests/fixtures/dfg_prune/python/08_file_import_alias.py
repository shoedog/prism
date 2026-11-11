from builtins import eval as ev


def f():
    x = 1
    x = ev("x + 1")
    return x
