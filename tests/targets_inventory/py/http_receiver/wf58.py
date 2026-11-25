from httpx import Client
from requests import Session
def fetch(u, client: "Client" = Session()):
    return client.get(u)
