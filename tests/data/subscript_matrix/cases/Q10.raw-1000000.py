def case():
    t = src()
    d = {}
    independent(t)
    d["a"] = t
    d[1] = "clean"
    d[2] = "clean"
    copy1 = d
    copy2 = d
    copy3 = d
    sink(d)

def independent(value):
    z = {}
    z[0] = value
    return z
