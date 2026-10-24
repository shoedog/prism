def f(value):
    x = source()
    match value:
        case Box(item=x):
            sink(x)
    sink(x)
