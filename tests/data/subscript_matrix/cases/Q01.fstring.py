def case():
    t = src()
    d = {}; k = "a"
    d[f"slot-{k}"] = t
    sink(d)
