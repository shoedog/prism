from typing import Optional
from httpx import Client
from requests import Session
def make() -> Optional[Client]:
    return Session()
def fetch(u):
    client = make()
    return client.get(u)
