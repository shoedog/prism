from typing import Optional
from httpx import Client
from requests import Session
def fetch(u, client: Optional[Client] = Session()):
    return client.get(u)
