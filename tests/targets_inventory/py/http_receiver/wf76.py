from requests import Session
def fetch(u):
    client, _ = Session(), None
    return client.get(u)
