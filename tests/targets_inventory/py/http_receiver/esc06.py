from requests import Session
Session.__new__ = staticmethod(lambda cls: {})
def fetch(u):
    return Session().get(u)
