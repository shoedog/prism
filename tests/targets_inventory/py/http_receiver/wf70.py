from httpx import Client
from requests import Session
def make() -> None | Client:
    return Session()
def fetch(u):
    client = make()
    return client.get(u)
