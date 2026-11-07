def f(v):
    x = 0
    match v:
        case 1:
            x = 1
            x = 2
            use(x)
        case 2:
            x = 3
            use(x)
        case _:
            use(x)
    use(x)
