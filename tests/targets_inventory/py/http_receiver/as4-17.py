from requests import Session
class Transport(Session):
    pass
def fetch(client: Transport, u):
    return client.get(u)
