def case():
    t = src()
    o = Box()
    o.items = [None]
    o.items[0] = t
    o.items += [None]
    sink(o)

class Box:
    pass
