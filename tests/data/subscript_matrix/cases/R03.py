def case():
    t = src()
    d = {"a": "s0", "b": "s0"}
    d["b"] = "s" # W
    f(d["b"]) # R

def f(p):
    sink(p)
