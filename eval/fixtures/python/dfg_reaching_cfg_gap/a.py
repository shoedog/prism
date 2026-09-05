def f():
    with make(
        source()
    ) as x:
        sink(x)
