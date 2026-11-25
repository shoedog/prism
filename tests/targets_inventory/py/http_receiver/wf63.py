from httpx import Client
from requests import Session
def make() -> Session:
    return Session()
def fetch(u):
    client: Client | None = make()
    return client.get(u)
