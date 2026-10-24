def f():
    x = source()
    try: risky()
    except Error as x:
        sink(x)
    sink(x)
