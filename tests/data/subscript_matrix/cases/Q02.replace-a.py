def case():
    t = src_a()
    u = src_b()
    d = {}
    e = {}
    d["a"] = t
    d["b"] = u
    d["a"] = "clean"
    sink(d)
