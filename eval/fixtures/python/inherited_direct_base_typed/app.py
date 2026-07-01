class Base:
    def go(self):
        pass

class Child(Base):
    pass

class Other:
    def go(self):
        pass

def run(c: Child):
    c.go()
