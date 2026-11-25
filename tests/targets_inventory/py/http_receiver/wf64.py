from typing import Union
from httpx import Client
from requests import Session
def make() -> Session:
    return Session()
def fetch(u):
    client: Union[Client, None] = make()
    return client.get(u)
