def f(value):
    x = source()
    match value:
        case Box(x):
            sink(x)
    sink(x)
