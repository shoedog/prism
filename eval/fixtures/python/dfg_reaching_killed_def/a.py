def f():
    global x
    x = source()
    x = clean()
    sink(x)
