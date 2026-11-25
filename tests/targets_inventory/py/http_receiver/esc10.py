from requests import Session
Session.__enter__ = lambda self: {}
def fetch(u):
    s = Session()
    return s.get(u)
