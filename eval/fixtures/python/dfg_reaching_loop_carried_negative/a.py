def f():
    x = source()
    while ready():
        sink(x)
        x = next_value(); x = clean()
