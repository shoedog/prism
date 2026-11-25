from requests import Session
session = Session()
def fetch(u, replacement):
    globals()["session"] = replacement
    return session.get(u)
