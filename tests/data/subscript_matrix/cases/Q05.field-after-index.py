def case():
    t = src()
    o = Box()
    o.items = [Box()]
    o.items[0].name = t
    sink(o)

class Box:
    pass
