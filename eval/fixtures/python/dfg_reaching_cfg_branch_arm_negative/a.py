def f(c):
    x = source()
    if c:
        x = clean()
    else:
        sink(x)
