from requests import Session
def fetch(u):
    try:
        raise RuntimeError(Session())
    except RuntimeError as client:
        return client.get(u)
