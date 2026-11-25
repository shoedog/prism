from httpx import Client
from requests import Session
def fetch(u, client: Session | Client = Session()):
    return client.get(u)
