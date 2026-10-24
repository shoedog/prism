def f(value):
    x = source()
    match value:
        case [y] as x:
            sink(x)
    sink(x)
