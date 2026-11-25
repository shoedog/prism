def case():
    t = src()
    d = {"a": "", "b": ""}
    d["a"] = t
    a1 = t
    a2 = a1
    a3 = a2
    a4 = a3
    a5 = a4
    copy = d
    joined = (copy, a5)
    sink(joined)
