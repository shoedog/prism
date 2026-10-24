def f(it):
    x = source()
    for x in it:
        sink(x)
    done()
    sink(x)
