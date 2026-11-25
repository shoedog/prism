from requests import Session
def init(self):
    self.get = {}.get
Session.__init__ = init
def fetch(u):
    s = Session()
    return s.get(u)
