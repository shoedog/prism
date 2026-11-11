def f():
    x = 1
    x = __import__("builtins").eval("x + 1")
    return x
