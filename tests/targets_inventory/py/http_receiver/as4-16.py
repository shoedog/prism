from requests import Session
def fetch(u, *clients: Session):
    return clients[0].get(u)
