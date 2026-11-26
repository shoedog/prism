from requests import Session
def fetch(u, replacement):
    session = Session()
    session.get = replacement
    return session.get(u)
