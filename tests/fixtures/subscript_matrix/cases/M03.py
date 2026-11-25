def case():
    t = src()
    o = C()
    p = o
    o.x = t # W
    p.x = "safe"
    sink(o.x) # R

class C:
    x = "s0"
