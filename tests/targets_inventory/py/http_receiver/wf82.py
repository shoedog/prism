from requests import Session
def fetch(u):
    for client in (Session(),):
        return client.get(u)
