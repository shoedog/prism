def case():
    t = src()
    d = {}
    d["a"] = t
    d |= {"b": None}
    sink(d)
