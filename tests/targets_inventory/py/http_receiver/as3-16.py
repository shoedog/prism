from requests import Session
def make(depth, u) -> Session:
    if depth:
        client = make(depth - 1, u)
        client.get(u)
        return client
    return Session()
