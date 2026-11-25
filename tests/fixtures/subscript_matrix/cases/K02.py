def case():
    t = src()
    d = D({"a": "s0"})
    d.a = "s0"
    d.a = t # W
    d["a"] = "safe"
    sink(d["a"]) # R

class D(dict):
    pass
