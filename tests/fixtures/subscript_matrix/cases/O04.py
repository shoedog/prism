def case():
    t = src()
    v = {"x": "s0"}
    d = {"a": v, "b": v}
    d["a"]["x"] = t # W
    sink(d["b"]["x"]) # R
