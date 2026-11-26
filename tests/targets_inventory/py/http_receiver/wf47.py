from requests import Session
def make() -> Session:
    return Session()
def fetch(u, replacement):
    client = make()
    client.__dict__["get"] = replacement
    return client.get(u)
