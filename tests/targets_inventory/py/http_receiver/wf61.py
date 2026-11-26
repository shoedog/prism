from typing import Optional as Maybe
from httpx import Client
from requests import Session
def make() -> Session:
    return Session()
def fetch(u):
    client: Maybe[Client] = make()
    return client.get(u)
