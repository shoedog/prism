def case():
    t = src()
    class C:
        pass
    o = C()
    o.a = "s0"
    o.a = t # W
    p = o
    sink(p.a) # R
