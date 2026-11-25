def case():
    t = src()
    i = "x"
    j = "y"
    d = {"x": {"a": "s0"}, "y": {"a": "s0"}}
    d[i]["a"] = t # W
    sink(d[j]["a"]) # R
