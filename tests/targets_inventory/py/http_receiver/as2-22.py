from requests import Session
def make(items) -> Session:
    return next(c for c in (yield from items))
def fetch(items, u):
    client = make(items)
    return client.get(u)
