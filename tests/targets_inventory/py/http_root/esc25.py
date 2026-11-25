from requests import Session
Session.__getattribute__ = lambda self,key: None
def fetch(s,u):
    return Session.get(s,u)
