def case():
    t = src()
    d = {}; e = {}
    d["a"] = t
    d[1] = "clean"
    d[2] = "clean"
    d[3] = "clean"
    d[4] = "clean"
    d[5] = "clean"
    d[6] = "clean"
    d[7] = "clean"
    d[8] = "clean"
    e[0] = t
    copy = e
    sink(d)
