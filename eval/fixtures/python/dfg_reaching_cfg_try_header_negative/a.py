def f():
    x = source()
    try:
        x = clean()
        raise Failure()
    except Failure:
        sink(x)
