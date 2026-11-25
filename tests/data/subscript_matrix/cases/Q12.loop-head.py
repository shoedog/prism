def case():
    t = src()
    d = {"a": "", "b": ""}
    for d["a"] in [t]:
        pass
    sink(d)
