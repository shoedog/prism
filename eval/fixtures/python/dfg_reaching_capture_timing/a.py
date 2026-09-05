def f():
    x = source()
    thunk = lambda: sink(x)
    x = clean()
    return thunk
