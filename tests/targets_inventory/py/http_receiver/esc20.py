from requests import Session
Session.get = lambda self,u: None
def fetch(u):
    s = Session()
    return s.get(u)
