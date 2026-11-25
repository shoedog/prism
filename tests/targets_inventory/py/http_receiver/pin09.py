from requests import Session
def replacement(req):
    return None
def make() -> Session:
    v = Session()
    d = v.__dict__
    d["send"] = replacement
    return v
def fetch(req):
    s = make()
    return s.send(req)
