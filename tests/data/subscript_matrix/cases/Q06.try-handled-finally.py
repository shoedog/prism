def case():
    t = src()
    d = {}
    d["a"] = t
    try:
        raise ValueError()
    except ValueError:
        spare = 1
    else:
        spare = 2
    finally:
        spare = 3
    sink(d)
