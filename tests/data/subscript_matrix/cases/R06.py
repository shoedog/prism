def case():
    t = src()
    d = {"a": "s0"}
    x = {}
    for i in range(2):
        if i == 1:
            x = d # R
            break
        d["a"] = t # W
    sink(x)
