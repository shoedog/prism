def case():
    t = src()
    d = {}
    try:
        d["a"] = t
        raise ValueError()
    except ValueError:
        sink(d)
