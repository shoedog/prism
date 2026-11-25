def case():
    t = src()
    d = {"a": "s0"}
    d["a"] = t # W
    def g():
        sink(d["a"]) # R
    g()
