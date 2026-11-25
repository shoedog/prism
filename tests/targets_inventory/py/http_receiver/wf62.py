from httpx import Client
from requests import Session
def fetch(u):
    client: None | Client = Session()
    return client.get(u)
