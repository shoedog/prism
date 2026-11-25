from requests import Session
Session.__enter__ = lambda self: {}
def fetch(u):
    with Session() as s:
        return s.get(u)
