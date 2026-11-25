def case():
    t = src()
    d = {}
    d["a"] = t
    x = d.get("a")
    sink(x)
