def case():
    t = src()
    d = {}
    d["a"] = t
    try:
        flag = True
    finally:
        flag = False
    sink(d)
