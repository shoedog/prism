def case():
    t = src()
    d = {}
    d["a"] = t
    local_a = None
    local_b = None
    sink(d)
