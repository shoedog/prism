def f(condition):
    x = source()
    if condition:
        import package as x
    sink(x)
