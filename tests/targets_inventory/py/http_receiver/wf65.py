from typing import Annotated
from httpx import Client
from requests import Session
def fetch(u):
    client: Annotated[Client, "transport"] = Session()
    return client.get(u)
