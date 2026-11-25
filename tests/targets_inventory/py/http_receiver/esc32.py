from requests import Session
registered = []
def register(obj):
    registered.append(obj)
def make() -> Session:
    v = Session()
    register(v)
    return v
def fetch(req):
    s = make()
    return s.send(req)
