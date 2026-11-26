def case():
    t = src()
    d = {"a": "s0"}
    d["a"], other = t, 1 # W
    x = d["a"] # R
    sink(x)
