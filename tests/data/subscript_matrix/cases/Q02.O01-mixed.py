def case():
    t = src_a()
    u = src_b()
    d = {}
    d["a"] = t
    d["b"] = u
    mutate(d)
    sink(d)

def mutate(value):
    value["a"] = "clean"
