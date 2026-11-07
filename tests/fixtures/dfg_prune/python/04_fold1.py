def same():
    x = 1; x = 2
    use(x)

def rebound(o, other):
    o.x = source()
    saved = o
    o = other
    o.x = 0
    o = saved
    sink(o.x)

def setter(self):
    self.value = 5
    self.value = None
    sink(self.value)
