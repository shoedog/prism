def sl():
    x = 1; x = 2
    use(x)

def rebind(o, other, saved):
    o.x = src()
    saved = o
    o = other
    o.x = 0
    o = saved
    sink(o.x)

def setter(self):
    self.v = 5
    self.v = None
    return self.v
