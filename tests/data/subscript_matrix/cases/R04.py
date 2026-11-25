def case():
    t = src()
    o = C()
    o.items = {"a": "s0"}
    o.items["a"] = t # W
    sink(o.items) # R

class C:
    pass
