def case():
    t = src()
    d = {}
    d["a"] = t
    local_a, local_b = 0, 1
    sink(d)
