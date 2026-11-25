def case():
    t = src()
    class C:
        pass
    o = C()
    o.a = "s0"
    o.a = t # W
    sink(o) # R
