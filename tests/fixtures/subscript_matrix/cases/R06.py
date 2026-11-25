def case():
    t = src()
    d = {"a": "s0"}
    for i in range(2):
        x = d["a"] # R
        d["a"] = t # W
    sink(x)
