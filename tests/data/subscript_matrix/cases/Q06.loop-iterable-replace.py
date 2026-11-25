def case():
    t = src()
    d = {}
    d["a"] = t
    for _ in (d := {}):
        pass
    sink(d)
