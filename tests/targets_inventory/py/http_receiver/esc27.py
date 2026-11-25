from requests import Session
class Base:
    def __init__(self):
        self.s = Session()
    def fetch(self,u):
        return self.s.get(u)
class Child(Base):
    def __init__(self,value):
        self.s = value
Child({}).fetch("u")
