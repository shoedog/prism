class C:
    def twice(self, a, b):
        self.x = a
        self.x = b
        use(self.x)

    def __new__(cls, a, b):
        cls.x = a
        cls.x = b
        use(cls.x)

    def rebound(self, a, other):
        self.x = a
        self = other
        self.x = a
        use(self.x)

    def nested_nonlocal(self, a, b):
        self.x = a
        def go():
            nonlocal self
            self = None
        self.x = b
        go()
        use(self.x)

    def deleted(self, a, b):
        self.x = a
        self.x = b
        del self
        use(self.x)

    def param_o(self, o, a, b):
        o.x = a
        o.x = b
        use(o.x)

    def prefix(self, a, b):
        self.x.y = a
        self.x = b
        use(self.x.y)

    def sibling(self, a, b):
        self.x = a
        self.y = b
        use(self.x)

    # Contract: self.<path> is fixed storage; a property setter or descriptor that
    # intercepts the write is out of model, as for Exact (`value = property(.., set_value)`).
    def set_value(self, v):
        self.value = 5
        self.value = None
        sink(self.value)

    def tf(self, a):
        self.x = a
        try:
            self.x = 2
        finally:
            use(self.x)

    def sup(self, a):
        self.x = a
        with suppress(E):
            self.x = f()
        use(self.x)

    def forelse(self, c):
        self.x = 1
        for i in c:
            self.x = 2
            break
        else:
            pass
        use(self.x)

    def gen(self):
        self.x = 1
        self.x = 2
        yield
        use(self.x)

    def exc(self):
        self.x = 1
        try:
            self.x = f()
        except E as self_err:
            use(self.x)

    def sameline(self):
        self.x = 1; self.x = 2
        use(self.x)

def free():
    self = make()
    self.x = 1
    self.x = 2
    use(self.x)

def with_param(self):
    self.x = 1
    self.x = 2
    use(self.x)

def asname(self, a, b):
    self.x = a
    with open() as self:
        pass
    self.x = b
    use(self.x)

def walrus(self, a, b):
    self.x = a
    if (self := b):
        pass
    self.x = b
    use(self.x)

def forself(self, a, b, c):
    self.x = a
    for self in c:
        pass
    self.x = b
    use(self.x)

def unpack(self, a, b):
    self.x = a
    self, k = b
    self.x = b
    use(self.x)

def glob(self, a, b):
    global self
    self.x = a
    self.x = b
    use(self.x)
