def case():
    t = src_a()
    u = src_b()
    d = {}
    d["a"] = t
    d["b"] = u
    e = d
    e["a"] = "clean"
    sink(d)
