def case():
    t = src()
    d = {}; flag = True
    d["a"] = t
    if flag:
        pass
    else:
        try:
            pass
        finally:
            pass
    sink(d)
