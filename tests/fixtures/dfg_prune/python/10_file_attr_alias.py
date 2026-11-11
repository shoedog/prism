import builtins
ev = builtins.eval


def f():
    x = 1
    x = ev("x + 1")
    return x
