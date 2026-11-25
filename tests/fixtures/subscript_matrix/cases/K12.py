def case():
    t = src()
    d = {"a": "s0", "b": "s0"}
    d[b"a"] = t # W
    d["a"] = "s"
    sink(d["a"]) # R
