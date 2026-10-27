def f(p):
    x = source()
    with open(p) as x:
        sink(x)
    sink(x)
