from requests import Session
def fetch(u, replacement):
    session = Session()
    alias = session
    alias.get = replacement
    return session.get(u)
