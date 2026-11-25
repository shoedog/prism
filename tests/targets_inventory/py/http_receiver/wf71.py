from typing import Annotated
from httpx import Client
from requests import Session
def make() -> Annotated[Client, "transport"]:
    return Session()
def fetch(u):
    client = make()
    return client.get(u)
