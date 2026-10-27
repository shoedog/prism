def f(value):
    x = source()
    match value:
        case (x,):
            sink(x)
    sink(x)
