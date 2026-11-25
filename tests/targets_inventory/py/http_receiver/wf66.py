from typing import Annotated
from httpx import Client
from requests import Session
def make() -> Session:
    return Session()
def fetch(u):
    client: Annotated[Client, "transport"] = make()
    return client.get(u)
